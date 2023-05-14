use gimli::write::{Address, Writer};
use gimli::{DW_EH_PE_omit, DW_EH_PE_uleb128, Encoding, LittleEndian};

#[derive(Debug)]
pub struct GccExceptTable {
    pub call_sites: CallSiteTable,
    pub actions: ActionTable,
    pub type_info: TypeInfoTable,
    pub exception_specs: ExceptionSpecTable,
}

impl GccExceptTable {
    pub fn write<W: Writer>(&self, w: &mut W, encoding: Encoding) -> gimli::write::Result<()> {
        // lpStartEncoding
        w.write_u8(DW_EH_PE_omit.0)?;
        // lpStart (omitted)
        let type_info_padding = if self.type_info.type_info.is_empty() {
            // ttypeEncoding
            w.write_u8(DW_EH_PE_omit.0)?;
            None
        } else {
            // ttypeEncoding
            w.write_u8(self.type_info.ttype_encoding.0)?;

            // classInfoOffset
            let class_info_offset_field_offset = w.len() as u64;

            // Note: The offset in classInfoOffset is relative to position right after classInfoOffset
            // itself.
            let class_info_offset_no_padding = self.call_sites.encoded_size()
                + self.actions.encoded_size()
                + self.type_info.encoded_size(encoding);

            let type_info_is_aligned = |type_info_padding: u64| {
                (class_info_offset_field_offset
                    + gimli::leb128::write::uleb128_size(
                        class_info_offset_no_padding + type_info_padding,
                    ) as u64
                    + self.call_sites.encoded_size()
                    + self.actions.encoded_size()
                    + type_info_padding)
                    % 4
                    == 0
            };

            let mut type_info_padding = 0;
            while !type_info_is_aligned(type_info_padding) {
                type_info_padding += 1;
            }

            w.write_uleb128(class_info_offset_no_padding + type_info_padding)?;

            Some(type_info_padding)
        };

        // call site table
        self.call_sites.write(w)?;

        // action table
        self.actions.write(w)?;

        // align to 4 bytes
        if let Some(type_info_padding) = type_info_padding {
            for _ in 0..type_info_padding {
                w.write_u8(0)?;
            }
            // In this case we calculated the expected padding amount and used it to write the
            // classInfoOffset field. Assert that the expected value matched the actual value to catch
            // any inconsistency.
            assert!(w.len() % 4 == 0, "type_info must be aligned to 4 bytes");
        } else {
            while w.len() % 4 != 0 {
                w.write_u8(0)?;
            }
        }

        // type_info
        self.type_info.write(w, encoding)?;

        // exception specs
        self.exception_specs.write(w)?;

        // align to 4 bytes
        while w.len() % 4 != 0 {
            w.write_u8(0)?;
        }

        Ok(())
    }
}

#[derive(Debug)]
pub struct CallSiteTable(pub Vec<CallSite>);

impl CallSiteTable {
    fn encoded_size(&self) -> u64 {
        let mut len = LenWriter(0);
        self.write(&mut len).unwrap();
        len.0 as u64
    }

    fn write<W: Writer>(&self, w: &mut W) -> gimli::write::Result<()> {
        let callsite_table_length = self.0.iter().map(|call_site| call_site.encoded_size()).sum();

        // callsiteEncoding
        w.write_u8(DW_EH_PE_uleb128.0)?;
        // callsiteTableLength
        w.write_uleb128(callsite_table_length)?;

        for call_site in &self.0 {
            call_site.write(w)?;
        }

        Ok(())
    }
}

#[derive(Debug)]
pub struct CallSite {
    pub start: u64,
    pub length: u64,
    pub landing_pad: u64,
    pub action_entry: Option<ActionOffset>,
}

impl CallSite {
    fn encoded_size(&self) -> u64 {
        let mut len = LenWriter(0);
        self.write(&mut len).unwrap();
        len.0 as u64
    }

    fn write<W: Writer>(&self, w: &mut W) -> gimli::write::Result<()> {
        w.write_uleb128(self.start)?;
        w.write_uleb128(self.length)?;
        w.write_uleb128(self.landing_pad)?;
        w.write_uleb128(match self.action_entry {
            Some(action_offset) => action_offset.0 + 1,
            None => 0,
        })?;
        Ok(())
    }
}

#[derive(Debug)]
pub struct ActionTable {
    actions: Vec<Action>,
    encoded_length: u64,
}

impl ActionTable {
    pub fn new() -> ActionTable {
        ActionTable { actions: vec![], encoded_length: 0 }
    }

    pub fn add(&mut self, action: Action) -> ActionOffset {
        let id = ActionOffset(self.encoded_length);
        self.encoded_length += action.encoded_size(self.encoded_length);
        self.actions.push(action);
        id
    }

    fn encoded_size(&self) -> u64 {
        let mut len = LenWriter(0);
        self.write(&mut len).unwrap();
        len.0 as u64
    }

    fn write<W: Writer>(&self, w: &mut W) -> gimli::write::Result<()> {
        let action_table_start = w.len() as u64;
        for action in &self.actions {
            action.write(w, w.len() as u64 - action_table_start)?;
        }

        Ok(())
    }
}

#[derive(Debug, Copy, Clone)]
pub struct ActionOffset(u64);

#[derive(Debug)]
pub struct Action {
    pub kind: ActionKind,
    pub next_action: Option<ActionOffset>,
}

impl Action {
    fn encoded_size(&self, action_table_offset: u64) -> u64 {
        let mut len = LenWriter(0);
        self.write(&mut len, action_table_offset).unwrap();
        len.0 as u64
    }

    fn write<W: Writer>(&self, w: &mut W, action_table_offset: u64) -> gimli::write::Result<()> {
        // ttypeIndex
        let ttype_index = match self.kind {
            ActionKind::Cleanup => 0,
            ActionKind::Catch(type_info_id) => type_info_id.0 as i64 + 1,
            ActionKind::ExceptionSpec(exception_spec_offset) => {
                -(exception_spec_offset.0 as i64 + 1)
            }
        };
        w.write_sleb128(ttype_index)?;
        // actionOffset
        let action_offset_field_offset =
            action_table_offset + gimli::leb128::write::sleb128_size(ttype_index) as u64;
        w.write_sleb128(match self.next_action {
            Some(next_action_offset) => {
                next_action_offset.0 as i64 - action_offset_field_offset as i64
            }
            None => 0,
        })?;
        Ok(())
    }
}

#[derive(Debug, Copy, Clone)]
pub enum ActionKind {
    Cleanup,
    Catch(TypeInfoId),
    // TODO
    ExceptionSpec(ExceptionSpecOffset),
}

#[derive(Debug)]
pub struct TypeInfoTable {
    ttype_encoding: gimli::DwEhPe,
    type_info: Vec<Address>,
}

impl TypeInfoTable {
    pub fn new(ttype_encoding: gimli::DwEhPe) -> TypeInfoTable {
        TypeInfoTable { ttype_encoding, type_info: vec![] }
    }

    pub fn add(&mut self, type_info: Address) -> TypeInfoId {
        let id = TypeInfoId(self.type_info.len() as u64);
        self.type_info.push(type_info);
        id
    }

    fn encoded_size(&self, encoding: Encoding) -> u64 {
        let mut len = LenWriter(0);
        self.write(&mut len, encoding).unwrap();
        len.0 as u64
    }

    fn write<W: Writer>(&self, w: &mut W, encoding: Encoding) -> gimli::write::Result<()> {
        for &type_info in self.type_info.iter().rev() {
            w.write_eh_pointer(type_info, self.ttype_encoding, encoding.address_size)?;
        }

        Ok(())
    }
}

#[derive(Debug, Copy, Clone)]
pub struct TypeInfoId(u64);

#[derive(Debug)]
pub struct ExceptionSpecTable {
    specs: Vec<ExceptionSpec>,
    encoded_length: u64,
}

impl ExceptionSpecTable {
    pub fn new() -> ExceptionSpecTable {
        ExceptionSpecTable { specs: vec![], encoded_length: 0 }
    }

    pub fn add(&mut self, exception_spec: ExceptionSpec) -> ExceptionSpecOffset {
        let id = ExceptionSpecOffset(self.encoded_length);
        self.encoded_length += exception_spec.encoded_size();
        self.specs.push(exception_spec);
        id
    }

    fn write<W: Writer>(&self, w: &mut W) -> gimli::write::Result<()> {
        for exception_spec in &self.specs {
            exception_spec.write(w)?;
        }

        Ok(())
    }
}

#[derive(Debug, Copy, Clone)]
pub struct ExceptionSpecOffset(u64);

#[derive(Debug)]
pub struct ExceptionSpec(pub Vec<TypeInfoId>);

impl ExceptionSpec {
    fn encoded_size(&self) -> u64 {
        let mut len = LenWriter(0);
        self.write(&mut len).unwrap();
        len.0 as u64
    }

    fn write<W: Writer>(&self, w: &mut W) -> gimli::write::Result<()> {
        for type_info_id in &self.0 {
            w.write_uleb128(type_info_id.0 + 1)?;
        }
        w.write_u8(0)
    }
}

struct LenWriter(usize);

impl Writer for LenWriter {
    type Endian = LittleEndian;

    fn endian(&self) -> LittleEndian {
        LittleEndian
    }

    fn len(&self) -> usize {
        self.0
    }

    fn write(&mut self, bytes: &[u8]) -> gimli::write::Result<()> {
        self.0 += bytes.len();
        Ok(())
    }

    fn write_at(&mut self, offset: usize, bytes: &[u8]) -> gimli::write::Result<()> {
        assert!(offset + bytes.len() < self.0);
        Ok(())
    }
}
