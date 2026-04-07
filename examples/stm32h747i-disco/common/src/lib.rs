#![no_std]

pub use heapless;

#[derive(Debug)]
pub enum LogLevel {
    Info,
    Warn,
}

/// Application message to send between the two cores
#[derive(Debug)]
pub enum Message {
    Ping(u32),
    Pong(u32),
    Log {
        level: LogLevel,
        message: heapless::String<128>,
    },
}
