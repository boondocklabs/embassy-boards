//! Memory Layout Configuration
//!
//! Const memory layout definitions for use with an associated type of the [`BoardConfig`] trait.
//! The layout associated with a board will automatically generate a `memory.x` layout at build time,
//! and const regions, sections, lengths, and MPU flags can be interrogated at runtime.
//!
//! Each region and section can have MPU attributes associated with them for auto MPU configuration.

use crate::str_eq;

pub mod region;
pub mod section;

pub trait BoardMemory {
    const MEMORY: MemoryLayout;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MpuAttrs {
    pub tex: u8,
    pub executable: bool,
    pub shareable: bool,
    pub cacheable: bool,
    pub bufferable: bool,
}

impl MpuAttrs {
    pub const fn new(
        tex: u8,
        executable: bool,
        shareable: bool,
        cacheable: bool,
        bufferable: bool,
    ) -> Self {
        Self {
            tex,
            executable,
            shareable,
            cacheable,
            bufferable,
        }
    }
}

/// Memory aliases are used to alias physical regions
/// For example on dual core targets, flash may be split between two cores
/// and defined in the base MemoryLayout, but compilation targets
/// receive aliases for FLASH and RAM which alias to their respective regions
/// for the linker
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MemoryAlias {
    pub name: &'static str,
    pub target: &'static str,
}

/// Resolved memory layout from a [`MemoryConfig`]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MemoryLayout {
    pub regions: &'static [region::MemoryRegionSpec],
    pub aliases: &'static [MemoryAlias],
}

impl MemoryLayout {
    pub const fn region(self, name: &str) -> Option<region::MemoryRegion> {
        let mut i = 0;
        while i < self.regions.len() {
            let region = self.regions[i];
            if str_eq(region.name, name) {
                return Some(region.resolve());
            }
            i += 1;
        }
        None
    }

    pub const fn alias(self, name: &str) -> Option<region::MemoryRegion> {
        let mut i = 0;
        while i < self.aliases.len() {
            let alias = self.aliases[i];
            if str_eq(alias.name, name) {
                return self.region(alias.target);
            }
            i += 1;
        }
        None
    }

    pub const fn section(
        self,
        region_name: &str,
        section_name: &str,
    ) -> Option<section::MemorySection> {
        let mut i = 0;
        while i < self.regions.len() {
            let region = self.regions[i];
            if str_eq(region.name, region_name) {
                return region.resolve_section(section_name);
            }
            i += 1;
        }
        None
    }
}

#[cfg(feature = "_build")]
fn resolve_sections(
    region: &region::MemoryRegionSpec,
) -> Vec<(&'static section::MemorySectionSpec, usize)> {
    let mut out = Vec::new();
    let mut cursor = 0usize;

    if let Some(sections) = region.sections {
        for section in sections {
            use crate::align_up;

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

#[cfg(feature = "_build")]
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

    fn find_region<'a>(layout: &'a MemoryLayout, name: &str) -> &'a region::MemoryRegionSpec {
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
