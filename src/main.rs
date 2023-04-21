use gimli::write::{
    Address, CommonInformationEntry, EhFrame, EndianVec, FrameDescriptionEntry, FrameTable, Writer,
};
use gimli::{
    DW_EH_PE_omit, DW_EH_PE_udata4, DW_EH_PE_uleb128, Encoding, Format, LittleEndian, Register,
};
use object::write::{Object, Relocation, Symbol, SymbolSection};
use object::{
    Architecture, BinaryFormat, Endianness, RelocationEncoding, RelocationKind, SectionKind,
    SymbolFlags, SymbolKind, SymbolScope,
};

fn main() {
    let mut obj = Object::new(BinaryFormat::Elf, Architecture::X86_64, Endianness::Little);

    let abort_symbol = obj.add_symbol(Symbol {
        name: b"_ZN3std7process5abort17hbff88e5512836284E".to_vec(),
        value: 0,
        size: 0,
        kind: SymbolKind::Text,
        scope: SymbolScope::Dynamic,
        weak: false,
        section: SymbolSection::Undefined,
        flags: SymbolFlags::None,
    });

    let drop_section = obj.add_section(
        b"".to_vec(),
        b".text._ZN4core3ptr37drop_in_place$LT$simple..DropBomb$GT$17h8ed806fca2c6cf3aE".to_vec(),
        SectionKind::Text,
    );
    let _drop_symbol = obj.add_symbol(Symbol {
        name: b"_ZN4core3ptr37drop_in_place$LT$simple..DropBomb$GT$17h8ed806fca2c6cf3aE".to_vec(),
        value: 0,
        size: 0,
        kind: SymbolKind::Text,
        scope: SymbolScope::Compilation,
        weak: false,
        section: SymbolSection::Section(drop_section),
        flags: SymbolFlags::None,
    });

    let main_section = obj.add_section(
        b"".to_vec(),
        b".text._ZN6simple4main17ha25659610fd14e78E".to_vec(),
        SectionKind::Text,
    );
    let _main_symbol = obj.add_symbol(Symbol {
        name: b"_ZN6simple4main17ha25659610fd14e78E".to_vec(),
        value: 0,
        size: 0,
        kind: SymbolKind::Text,
        scope: SymbolScope::Compilation,
        weak: false,
        section: SymbolSection::Section(main_section),
        flags: SymbolFlags::None,
    });

    // 0000000000000000 <_ZN4core3ptr37drop_in_place$LT$simple..DropBomb$GT$17h8ed806fca2c6cf3aE>:
    //    0:   50                      push   %rax
    //    1:   ff 15 00 00 00 00       callq  *0x0(%rip)        # 7 <_ZN4core3ptr37drop_in_place$LT$simple..DropBomb$GT$17h8ed806fca2c6cf3aE+0x7>
    //                         3: R_X86_64_GOTPCREL    _ZN3std7process5abort17hbff88e5512836284E-0x4
    //    7:   0f 0b                   ud2
    obj.append_section_data(
        drop_section,
        &[0x50, 0xff, 0x15, 0, 0, 0, 0, 0x0f, 0x0b],
        0x10,
    );
    obj.add_relocation(
        drop_section,
        Relocation {
            offset: 3,
            size: 32,
            kind: RelocationKind::GotRelative,
            encoding: RelocationEncoding::Generic,
            symbol: abort_symbol,
            addend: -4,
        },
    )
    .unwrap();

    // 0000000000000000 <_ZN6simple4main17ha25659610fd14e78E>:
    //    0:   50                      push   %rax
    //    1:   48 8d 15 00 00 00 00    lea    0x0(%rip),%rdx        # 8 <_ZN6simple4main17ha25659610fd14e78E+0x8>
    //                         4: R_X86_64_PC32        .data.rel.ro..Lanon.fad58de7366495db4650cfefac2fcd61.1-0x4
    //    8:   bf 01 00 00 00          mov    $0x1,%edi
    //    d:   be 01 00 00 00          mov    $0x1,%esi
    //   12:   ff 15 00 00 00 00       callq  *0x0(%rip)        # 18 <_ZN6simple4main17ha25659610fd14e78E+0x18>
    //                         14: R_X86_64_GOTPCREL   _ZN4core9panicking18panic_bounds_check17h7240348ec330e47eE-0x4
    //   18:   eb 05                   jmp    1f <_ZN6simple4main17ha25659610fd14e78E+0x1f>
    //   1a:   e8 00 00 00 00          callq  1f <_ZN6simple4main17ha25659610fd14e78E+0x1f>
    //                         1b: R_X86_64_PLT32      .text._ZN4core3ptr37drop_in_place$LT$simple..DropBomb$GT$17h8ed806fca2c6cf3aE-0x4
    //   1f:   0f 0b                   ud2
    //   21:   ff 15 00 00 00 00       callq  *0x0(%rip)        # 27 <_ZN6simple4main17ha25659610fd14e78E+0x27>
    //                         23: R_X86_64_GOTPCREL   _ZN4core9panicking19panic_cannot_unwind17h42344709dec62f2eE-0x4
    //   27:   0f 0b                   ud2
    obj.append_section_data(
        main_section,
        &[
            0x50, 0x48, 0x8d, 0x15, 0, 0, 0, 0, 0xbf, 0x01, 0, 0, 0, 0xbe, 0x01, 0, 0, 0, 0xff,
            0x15, 0, 0, 0, 0, 0xeb, 0x05, 0xe8, 0, 0, 0, 0, 0x0f, 0x0b, 0xff, 0x15, 0, 0, 0, 0,
            0x0f, 0x0b,
        ],
        0x10,
    );

    let mut frame_table = FrameTable::default();
    let encoding = Encoding {
        address_size: 8,
        format: Format::Dwarf32,
        version: 1,
    };
    let mut cie = CommonInformationEntry::new(encoding, 4, -8, Register(0));
    cie.fde_address_encoding = gimli::DwEhPe(gimli::DW_EH_PE_pcrel.0 | gimli::DW_EH_PE_sdata4.0);
    cie.lsda_encoding = Some(gimli::DwEhPe(
        gimli::DW_EH_PE_pcrel.0 | gimli::DW_EH_PE_sdata4.0,
    ));
    cie.personality = Some((
        gimli::DwEhPe(gimli::DW_EH_PE_pcrel.0 | gimli::DW_EH_PE_sdata4.0),
        Address::Constant(42),
    ));
    let cie = frame_table.add_cie(cie);

    let mut fde = FrameDescriptionEntry::new(Address::Constant(10), 1);
    fde.lsda = Some(Address::Constant(11));
    frame_table.add_fde(cie, fde);

    let mut eh_frame = EhFrame::from(EndianVec::new(LittleEndian));
    frame_table.write_eh_frame(&mut eh_frame).unwrap();

    let eh_frame_section = obj.add_section(
        b"".to_vec(),
        b".eh_frame".to_vec(),
        SectionKind::ReadOnlyData,
    );
    obj.append_section_data(eh_frame_section, &eh_frame.0.into_vec(), 8);

    let mut type_info = TypeInfoTable::new(DW_EH_PE_udata4);
    let type1 = type_info.add(Address::Constant(0));
    let type2 = type_info.add(Address::Constant(0));
    let type3 = type_info.add(Address::Constant(0));

    let mut exception_specs = ExceptionSpecTable::new();
    let exception_spec_id1 = exception_specs.add(ExceptionSpec(vec![type1, type2]));

    let mut actions = ActionTable::new();
    let action1 = actions.add(Action {
        kind: ActionKind::ExceptionSpec(exception_spec_id1),
        next_action: None,
    });
    let action2 = actions.add(Action {
        kind: ActionKind::Cleanup,
        next_action: None,
    });
    let action3 = actions.add(Action {
        kind: ActionKind::ExceptionSpec(exception_spec_id1),
        next_action: Some(action2),
    });
    let action4 = actions.add(Action {
        kind: ActionKind::Catch(type3),
        next_action: Some(action3),
    });

    let gcc_except_table_data = GccExceptTable {
        call_sites: CallSiteTable(vec![
            CallSite {
                start: 0x08,
                length: 0x09,
                landing_pad: 0x25,
                action_entry: Some(action1),
            },
            CallSite {
                start: 0x16,
                length: 0x05,
                landing_pad: 0x31,
                action_entry: Some(action4),
            },
            CallSite {
                start: 0x1b,
                length: 0x66,
                landing_pad: 0,
                action_entry: None,
            },
        ]),
        actions,
        type_info,
        exception_specs,
    };

    let mut gcc_except_table = EndianVec::new(LittleEndian);

    // lpStartEncoding
    gcc_except_table.write_u8(DW_EH_PE_omit.0).unwrap();
    // lpStart (omitted)
    let type_info_padding = if gcc_except_table_data.type_info.type_info.is_empty() {
        // ttypeEncoding
        gcc_except_table.write_u8(DW_EH_PE_omit.0).unwrap();
        None
    } else {
        // ttypeEncoding
        gcc_except_table
            .write_u8(gcc_except_table_data.type_info.ttype_encoding.0)
            .unwrap();

        // classInfoOffset
        let class_info_offset_field_offset = gcc_except_table.len() as u64;

        // Note: The offset in classInfoOffset is relative to position right after classInfoOffset
        // itself.
        let class_info_offset_no_padding = gcc_except_table_data.call_sites.encoded_size()
            + gcc_except_table_data.actions.encoded_size()
            + gcc_except_table_data.type_info.encoded_size(encoding);

        let type_info_is_aligned = |type_info_padding: u64| {
            (class_info_offset_field_offset
                + gimli::leb128::write::uleb128_size(
                    class_info_offset_no_padding + type_info_padding,
                ) as u64
                + gcc_except_table_data.call_sites.encoded_size()
                + gcc_except_table_data.actions.encoded_size()
                + type_info_padding)
                % 4
                == 0
        };

        let mut type_info_padding = 0;
        while !type_info_is_aligned(type_info_padding) {
            type_info_padding += 1;
        }

        gcc_except_table
            .write_uleb128(class_info_offset_no_padding + type_info_padding)
            .unwrap();

        Some(type_info_padding)
    };

    // call site table
    gcc_except_table_data
        .call_sites
        .write(&mut gcc_except_table)
        .unwrap();

    // action table
    gcc_except_table_data
        .actions
        .write(&mut gcc_except_table)
        .unwrap();

    // align to 4 bytes
    if let Some(type_info_padding) = type_info_padding {
        for _ in 0..type_info_padding {
            gcc_except_table.write_u8(0).unwrap();
        }
        // In this case we calculated the expected padding amount and used it to write the
        // classInfoOffset field. Assert that the expected value matched the actual value to catch
        // any inconsistency.
        assert!(
            gcc_except_table.len() % 4 == 0,
            "type_info must be aligned to 4 bytes"
        );
    } else {
        while gcc_except_table.len() % 4 != 0 {
            gcc_except_table.write_u8(0).unwrap();
        }
    }

    // type_info
    gcc_except_table_data
        .type_info
        .write(&mut gcc_except_table, encoding)
        .unwrap();

    // exception specs
    gcc_except_table_data
        .exception_specs
        .write(&mut gcc_except_table)
        .unwrap();

    // TODO exception specifications

    // align to 4 bytes
    while gcc_except_table.len() % 4 != 0 {
        gcc_except_table.write_u8(0).unwrap();
    }

    let gcc_except_table_section = obj.add_section(
        b"".to_vec(),
        b".gcc_except_table".to_vec(),
        SectionKind::ReadOnlyData,
    );
    obj.append_section_data(gcc_except_table_section, &gcc_except_table.into_vec(), 8);

    std::fs::write("foo.o", obj.write().unwrap()).unwrap();
}

struct GccExceptTable {
    call_sites: CallSiteTable,
    actions: ActionTable,
    type_info: TypeInfoTable,
    exception_specs: ExceptionSpecTable,
}

struct CallSiteTable(Vec<CallSite>);

impl CallSiteTable {
    fn encoded_size(&self) -> u64 {
        let mut len = LenWriter(0);
        self.write(&mut len).unwrap();
        len.0 as u64
    }

    fn write<W: Writer>(&self, w: &mut W) -> gimli::write::Result<()> {
        let callsite_table_length = self
            .0
            .iter()
            .map(|call_site| call_site.encoded_size())
            .sum();

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

struct CallSite {
    start: u64,
    length: u64,
    landing_pad: u64,
    action_entry: Option<ActionOffset>,
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

struct ActionTable {
    actions: Vec<Action>,
    encoded_length: u64,
}

impl ActionTable {
    fn new() -> ActionTable {
        ActionTable {
            actions: vec![],
            encoded_length: 0,
        }
    }

    fn add(&mut self, action: Action) -> ActionOffset {
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

#[derive(Copy, Clone)]
struct ActionOffset(u64);

struct Action {
    kind: ActionKind,
    next_action: Option<ActionOffset>,
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

#[derive(Copy, Clone)]
enum ActionKind {
    Cleanup,
    Catch(TypeInfoId),
    // TODO
    ExceptionSpec(ExceptionSpecOffset),
}

struct TypeInfoTable {
    ttype_encoding: gimli::DwEhPe,
    type_info: Vec<Address>,
}

impl TypeInfoTable {
    fn new(ttype_encoding: gimli::DwEhPe) -> TypeInfoTable {
        TypeInfoTable {
            ttype_encoding,
            type_info: vec![],
        }
    }

    fn add(&mut self, type_info: Address) -> TypeInfoId {
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

#[derive(Copy, Clone)]
struct TypeInfoId(u64);

struct ExceptionSpecTable {
    specs: Vec<ExceptionSpec>,
    encoded_length: u64,
}

impl ExceptionSpecTable {
    fn new() -> ExceptionSpecTable {
        ExceptionSpecTable {
            specs: vec![],
            encoded_length: 0,
        }
    }

    fn add(&mut self, exception_spec: ExceptionSpec) -> ExceptionSpecOffset {
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

#[derive(Copy, Clone)]
struct ExceptionSpecOffset(u64);

struct ExceptionSpec(Vec<TypeInfoId>);

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
