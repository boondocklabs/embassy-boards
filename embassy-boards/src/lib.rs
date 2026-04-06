#![cfg_attr(not(feature = "_build"), no_std)]

use crate::memory::BoardMemory;

#[cfg(feature = "_stm32")]
pub mod bsp {
    mod stm32;
    #[cfg(not(feature = "_build"))]
    pub use stm32::BSP;
    pub use stm32::Board;
}

#[cfg(feature = "_runtime")]
pub use embassy_executor;

#[cfg(feature = "_runtime")]
pub use cortex_m;

#[cfg(feature = "_runtime")]
pub use cortex_m_rt;

#[cfg(feature = "_runtime")]
pub use embassy_time;

#[cfg(all(feature = "_runtime", feature = "_stm32"))]
pub use embassy_stm32;

#[cfg(feature = "_runtime")]
pub mod drivers;

#[cfg(all(feature = "_runtime", feature = "display"))]
pub use embedded_graphics;

#[cfg(all(feature = "_runtime", feature = "terminal"))]
pub use ratatui;

#[cfg(all(feature = "_runtime", feature = "defmt"))]
pub use defmt;

#[cfg(all(not(feature = "_build"), feature = "display"))]
pub mod display;

pub mod memory;

/// Board Support Trait
#[allow(async_fn_in_trait)]
pub trait BoardConfig {
    const NAME: &str;
    const VENDOR: &str;

    /// Memory layout
    type Layout: BoardMemory;

    /// Devices returned by `init()`
    type Devices;

    /// Initialize the board
    async fn init() -> Self::Devices;
}

/// Align to next highest alignment boundary
const fn align_up(x: usize, align: usize) -> usize {
    debug_assert!(align.is_power_of_two());
    (x + (align - 1)) & !(align - 1)
}

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
