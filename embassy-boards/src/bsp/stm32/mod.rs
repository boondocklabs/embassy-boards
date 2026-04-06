//! STM32 Boards

#[cfg(not(feature = "_build"))]
mod runtime;

#[cfg(feature = "board-stm32h747i-disco")]
mod stm32h747i_disco;
#[cfg(all(feature = "board-stm32h747i-disco", feature = "cpu1"))]
pub use stm32h747i_disco::Stm32h747iCm4Memory as Board;
#[cfg(all(feature = "board-stm32h747i-disco", feature = "cpu0"))]
pub use stm32h747i_disco::Stm32h747iCm7Memory as Board;
#[cfg(feature = "stm32h747i-disco-cm7")]
pub use stm32h747i_disco::cm7::Board as BSP;
#[cfg(feature = "stm32h747i-disco-cm4")]
pub use stm32h747i_disco_cm7::Stm32h747iDiscoSecondary as BSP;
