//! STM32H747I-DISCO

#[cfg(feature = "stm32h747i-disco-cm7")]
pub mod cm7;

#[cfg(feature = "stm32h747i-disco-cm4")]
pub mod cm4;

mod memory;

/// Memory layout for the CM7

#[cfg(all(feature = "board-stm32h747i-disco", feature = "cpu0"))]
pub struct Stm32h747iCm7Memory {}

/// Memory layout for the CM4
#[cfg(all(feature = "board-stm32h747i-disco", feature = "cpu1"))]
pub struct Stm32h747iCm4Memory {}
