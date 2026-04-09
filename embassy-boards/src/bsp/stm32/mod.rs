//! STM32 Boards

mod runtime;

#[cfg(feature = "dual-core")]
mod shared_queue;

#[cfg(board = "board-stm32h747i-disco")]
mod stm32h747i_disco;

#[cfg(board = "board-stm32f429i-disco")]
mod stm32f429i_disco;
