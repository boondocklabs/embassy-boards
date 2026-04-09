//! Core board support crate
//!
//! * Data structures shared by both config and runtime environments
#![no_std]

pub mod memory;

pub mod prelude {
    pub use super::memory::BoardMemory;
    pub use super::memory::MemoryAlias;
    pub use super::memory::MemoryLayout;
    pub use super::memory::MpuAttrs;
    pub use super::memory::region::{MemoryRegionSpec, RegionKind};
    pub use super::memory::section::MemorySectionSpec;
}

/// Align to next highest alignment boundary
pub const fn align_up(x: usize, align: usize) -> usize {
    debug_assert!(align.is_power_of_two());
    (x + (align - 1)) & !(align - 1)
}

/// Const string comparison
pub const fn str_eq(a: &str, b: &str) -> bool {
    let a = a.as_bytes();
    let b = b.as_bytes();

    if a.len() != b.len() {
        return false;
    }

    let mut i = 0;
    while i < a.len() {
        if a[i] != b[i] {
            return false;
        }
        i += 1;
    }

    true
}
