//#![cfg_attr(feature = "_runtime", no_std)]
#![no_std]
#![allow(unexpected_cfgs)]

mod memory {
    include!(concat!(env!("OUT_DIR"), "/memory.rs"));
}

/// Memory struct which implements [`BoardMemory`] generated from embassy-boards-config in build.rs
pub struct Memory {}

pub struct Board {}

#[cfg(platform = "stm32")]
pub mod bsp {
    mod stm32;
}

#[cfg(platform = "rp")]
pub mod bsp {
    mod rp;
}

use embassy_boards_core::memory::BoardMemory;
pub use embassy_executor;

pub use cortex_m;
pub use cortex_m_rt;

pub use embassy_time;

pub use embassy_sync;

#[cfg(platform = "stm32")]
pub use embassy_stm32;

#[cfg(platform = "rp")]
pub use embassy_rp;

pub mod drivers;

#[cfg(all(feature = "display"))]
pub use embedded_graphics;

#[cfg(all(feature = "pmod"))]
pub use embedded_hal_bus;

#[cfg(all(feature = "terminal"))]
pub use ratatui;

#[cfg(all(feature = "defmt"))]
pub use defmt;

#[cfg(all(feature = "display"))]
pub mod display;

/// Board Support Trait
#[allow(async_fn_in_trait)]
pub trait BoardConfig {
    const NAME: &str;
    const VENDOR: &str;

    /// Memory layout
    type Layout: BoardMemory;

    /// Devices returned by `init()`
    type Devices;

    /// Inter-core message type for dual-core boards
    #[cfg(feature = "dual-core")]
    type Message;

    /// Initialize the board
    async fn init() -> Self::Devices;

    #[cfg(feature = "heap")]
    /// Return an estimate of free heap memory
    fn heap_free() -> usize;

    #[cfg(feature = "heap")]
    /// Return an estimate of used heap memory
    fn heap_used() -> usize;

    #[cfg(feature = "heap")]
    /// Return the total heap memory size
    fn heap_size() -> usize;
}
