use crate::align_up;
use crate::str_eq;

use super::section::MemorySection;
use super::section::MemorySectionSpec;

use super::MpuAttrs;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RegionKind {
    Flash,
    Ram,
    ExternalRam,
    ExternalFlash,
    Reserved,
}

/// Resolved memory region
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MemoryRegion {
    pub name: &'static str,
    pub origin: usize,
    pub length: usize,
    pub kind: RegionKind,
    pub mpu: Option<MpuAttrs>,
    pub sections: Option<&'static [MemorySection]>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MemoryRegionSpec {
    pub name: &'static str,
    pub origin: usize,
    pub length: usize,
    pub kind: RegionKind,
    pub mpu: Option<MpuAttrs>,
    pub sections: Option<&'static [MemorySectionSpec]>,
}

impl MemoryRegionSpec {
    /// Find a [`MemorySection`] by name
    pub fn section(&self, name: &str) -> Option<MemorySectionSpec> {
        if let Some(&section) = self.sections?.iter().find(|p| p.name == name) {
            return Some(section.with_offset(0));
        }
        None
    }
}

impl MemoryRegionSpec {
    pub const fn resolve_section_index(&self, index: usize) -> Option<MemorySection> {
        let sections = match self.sections {
            Some(sections) => sections,
            None => return None,
        };

        if index >= sections.len() {
            return None;
        }

        let mut cursor = 0usize;
        let mut i = 0;
        while i < sections.len() {
            let section = sections[i];
            let align = if section.align == 0 { 1 } else { section.align };

            let offset = match section.offset {
                Some(offset) => offset,
                None => align_up(cursor, align),
            };

            if offset + section.length > self.length {
                panic!("section out of bounds");
            }

            if i == index {
                return Some(MemorySection {
                    name: section.name,
                    origin: self.origin + offset,
                    length: section.length,
                    align,
                    mpu: section.mpu,
                });
            }

            cursor = offset + section.length;
            i += 1;
        }

        None
    }

    pub const fn resolve_section(&self, name: &str) -> Option<MemorySection> {
        let sections = match self.sections {
            Some(sections) => sections,
            None => return None,
        };

        let mut cursor = 0usize;
        let mut i = 0;
        while i < sections.len() {
            let section = sections[i];
            let align = if section.align == 0 { 1 } else { section.align };

            let offset = match section.offset {
                Some(offset) => offset,
                None => align_up(cursor, align),
            };

            if offset + section.length > self.length {
                panic!("section out of bounds");
            }

            if str_eq(section.name, name) {
                return Some(MemorySection {
                    name: section.name,
                    origin: self.origin + offset,
                    length: section.length,
                    align,
                    mpu: section.mpu,
                });
            }

            cursor = offset + section.length;
            i += 1;
        }

        None
    }

    pub const fn resolve(self) -> MemoryRegion {
        MemoryRegion {
            name: self.name,
            origin: self.origin,
            length: self.length,
            kind: self.kind,
            mpu: self.mpu,
            sections: None,
        }
    }
}
