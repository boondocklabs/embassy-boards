#![no_std]
#![no_main]

use boards::BoardConfig;
use boards::bsp::BSP;
use boards::cortex_m_rt;
use embassy_boards as boards;
use embassy_boards::embassy_time::Timer;
#[cfg(feature = "terminal")]
use embassy_boards::ratatui::style::Stylize;
#[cfg(feature = "terminal")]
use embassy_boards::ratatui::widgets::{Block, Paragraph};
use embassy_executor::Spawner;

extern crate alloc;

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let mut devices = BSP::init().await;

    devices.network.start(spawner.clone()).await;

    defmt::info!("{} initialized", BSP::NAME);

    let mut count = 0u32;
    loop {
        #[cfg(feature = "terminal")]
        devices
            .terminal
            .draw(|frame| {
                use alloc::format;
                use alloc::vec;
                use embassy_boards::ratatui::widgets::BorderType;

                frame.render_widget(
                    Paragraph::new(vec![
                        "Hello, Embassy".light_cyan().into(),
                        format!("{}", BSP::NAME).yellow().into(),
                        format!("Heap free: {} KiB", BSP::heap_free() / 1024).into(),
                        format!("Counter: {}", count).magenta().into(),
                    ])
                    .block(
                        Block::bordered()
                            .border_type(BorderType::Rounded)
                            .title(" Embassy Boards "),
                    ),
                    frame.area(),
                )
            })
            .await
            .unwrap();
        devices.led.mask(count);
        count = count.wrapping_add(1);
        Timer::after_millis(50).await;
    }
}
