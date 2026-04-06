#![no_std]
#![no_main]

extern crate alloc;

use alloc::format;
use alloc::vec;
use boards::BoardConfig;
use boards::cortex_m_rt;
use boards::drivers::BoardDrivers;
use boards::drivers::touch::TouchPoint;
use boards::ratatui::widgets::Paragraph;
use embassy_executor::Spawner;

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let mut devices = boards::bsp::BSP::init().await;
    defmt::info!("Board {} initialized", boards::bsp::BSP::NAME);

    spawner.spawn(touch_task(devices.touch).unwrap());

    let mut counter = 0u32;

    loop {
        devices
            .terminal
            .draw(|frame| {
                frame.render_widget(
                    Paragraph::new(vec![format!("Counter: {}", counter).into()]),
                    frame.area(),
                );
            })
            .await
            .unwrap();

        counter = counter.wrapping_add(1);

        devices.dsi.wait_refresh().await;
    }
}

#[embassy_executor::task]
async fn touch_task(mut touch: <boards::bsp::BSP as BoardDrivers>::Touch) {
    let mut touches: [Option<TouchPoint>; 2] = [None; 2];
    loop {
        match touch.read_touches(&mut touches).await {
            Ok(count) => {
                for i in 0..count {
                    defmt::info!("touch: {}", touches[i])
                }
            }
            Err(e) => defmt::error!("Touch error: {}", e),
        }
    }
}
