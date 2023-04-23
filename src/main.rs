use gimli::write::{
    Address, CommonInformationEntry, EhFrame, EndianVec, FrameDescriptionEntry, FrameTable,
};
use gimli::{DW_EH_PE_udata4, Encoding, Format, LittleEndian, Register};
use object::write::{Object, Relocation, Symbol, SymbolSection};
use object::{
    Architecture, BinaryFormat, Endianness, RelocationEncoding, RelocationKind, SectionKind,
    SymbolFlags, SymbolKind, SymbolScope,
};

use eh_frame_experiments::*;

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
    //    8:   bf 01 00 00 00          mov    x1,%edi
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

    gcc_except_table_data
        .write(&mut gcc_except_table, encoding)
        .unwrap();

    let gcc_except_table_section = obj.add_section(
        b"".to_vec(),
        b".gcc_except_table".to_vec(),
        SectionKind::ReadOnlyData,
    );
    obj.append_section_data(gcc_except_table_section, &gcc_except_table.into_vec(), 8);

    std::fs::write("foo.o", obj.write().unwrap()).unwrap();
}
