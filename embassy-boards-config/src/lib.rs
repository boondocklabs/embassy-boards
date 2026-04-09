mod parser;
pub mod prelude {
    pub use super::parser::Boards;
    pub use embassy_boards_core::memory::BoardMemory;
}

pub mod error;
pub mod memory;
