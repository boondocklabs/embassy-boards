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
