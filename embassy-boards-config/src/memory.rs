use embassy_boards_core::{
    align_up,
    memory::{MemoryLayout, region::MemoryRegionSpec, section::MemorySectionSpec},
};

/// Generate a `memory.x` linker script from a [`MemoryLayout`]
pub fn generate_memory_linker(layout: &MemoryLayout) -> String {
    use core::fmt::Write;

    fn section_name(name: &str) -> String {
        let mut s = name.to_ascii_lowercase();
        s = s.replace([' ', '-', '/'], "_");
        if !s.starts_with('.') {
            s.insert(0, '.');
        }
        s
    }

    fn find_region<'a>(layout: &'a MemoryLayout, name: &str) -> &'a MemoryRegionSpec {
        layout
            .regions
            .iter()
            .find(|r| r.name == name)
            .unwrap_or_else(|| panic!("alias target region '{}' not found", name))
    }

    // Validate aliases don't collide with physical region names
    for alias in layout.aliases {
        assert!(
            !layout.regions.iter().any(|r| r.name == alias.name),
            "alias '{}' collides with physical region name",
            alias.name
        );
        let _ = find_region(layout, alias.target);
    }

    let mut out = String::new();

    // ----- MEMORY -----
    out.push_str("MEMORY\n{\n");

    // Physical regions
    for r in layout.regions {
        if r.length % 1024 == 0 {
            let _ = writeln!(
                out,
                "  {} : ORIGIN = 0x{:08X}, LENGTH = {}K",
                r.name,
                r.origin,
                r.length / 1024
            );
        } else {
            let _ = writeln!(
                out,
                "  {} : ORIGIN = 0x{:08X}, LENGTH = {}",
                r.name, r.origin, r.length
            );
        }
    }

    // Aliases
    for alias in layout.aliases {
        let target = find_region(layout, alias.target);

        if target.length % 1024 == 0 {
            let _ = writeln!(
                out,
                "  {} : ORIGIN = 0x{:08X}, LENGTH = {}K",
                alias.name,
                target.origin,
                target.length / 1024
            );
        } else {
            let _ = writeln!(
                out,
                "  {} : ORIGIN = 0x{:08X}, LENGTH = {}",
                alias.name, target.origin, target.length
            );
        }
    }

    out.push_str("}\n\n");

    // ----- SECTIONS -----
    out.push_str("SECTIONS\n{\n");

    for region in layout.regions {
        for (section, offset) in resolve_sections(region) {
            let abs = region.origin + offset;
            let sec = section_name(section.name);

            let _ = writeln!(
                out,
                "  {} 0x{:08X} (NOLOAD) :\n  {{\n    . = ALIGN({});\n    KEEP(*({}));\n    . = ALIGN({});\n  }} > {}\n",
                sec,
                abs,
                section.align.max(1),
                sec,
                section.align.max(1),
                region.name
            );
        }
    }

    out.push_str("}\n");
    out
}

fn resolve_sections(region: &MemoryRegionSpec) -> Vec<(&'static MemorySectionSpec, usize)> {
    let mut out = Vec::new();
    let mut cursor = 0usize;

    if let Some(sections) = region.sections {
        for section in sections {
            use align_up;

            let align = if section.align == 0 { 1 } else { section.align };

            let offset = match section.offset {
                Some(off) => off,
                None => align_up(cursor, align) as usize,
            };

            assert!(
                offset + section.length <= region.length,
                "section '{}' out of bounds in region '{}'",
                section.name,
                region.name
            );

            out.push((section, offset));
            cursor = offset + section.length;
        }
    }

    out
}
