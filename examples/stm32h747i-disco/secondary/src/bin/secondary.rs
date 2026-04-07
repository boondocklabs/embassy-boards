#![no_std]
#![no_main]

use boards::BoardConfig;
use boards::bsp::BSP;
use boards::cortex_m_rt;
use common::Message;
use common::heapless::{String, format};
use embassy_boards as boards;
use embassy_boards::drivers::BoardDrivers;
use embassy_boards::embassy_stm32::cryp::{self, AesEcb};
use embassy_boards::embassy_time::Timer;
use embassy_executor::Spawner;

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let mut devices = BSP::<Message>::init().await;

    // Send a log message to the CM7
    devices
        .sender
        .send(Message::Log {
            level: common::LogLevel::Info,
            message: format!(128; "{} initialized", BSP::<Message>::NAME).unwrap(),
        })
        .await
        .ok();

    spawner.spawn(message_task(devices.sender).unwrap());

    // Toggle LEDs
    let mut count = 0u32;
    loop {
        devices.led.mask(count);
        count = count.wrapping_add(1);
        Timer::after_millis(100).await;
    }
}

#[embassy_executor::task]
async fn message_task(mut sender: <BSP<Message> as BoardDrivers>::Sender) {
    let mut sequence = 0u32;
    loop {
        //sender.send(Message::Ping(sequence)).await.ok();
        sender.send(Message::Ping(sequence)).await.ok();
        sequence = sequence.wrapping_add(1);
        Timer::after_millis(2).await;
    }
}
