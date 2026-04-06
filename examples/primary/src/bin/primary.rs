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
use embassy_boards as boards;
use embassy_boards::display::log_widget::LogWidget;
use embassy_boards::ratatui::layout::Constraint;
use embassy_boards::ratatui::layout::Layout;
use embassy_boards::ratatui::style::Stylize;
use embassy_boards::ratatui::widgets::Block;
use embassy_executor::Spawner;

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let mut devices = boards::bsp::BSP::init().await;
    defmt::info!("Board {} initialized", boards::bsp::BSP::NAME);

    spawner.spawn(touch_task(devices.touch).unwrap());

    let mut counter = 0u32;

    let logs = LogWidget::default();
    tracing::subscriber::set_global_default(logs.clone()).unwrap();

    tracing::info!("Board {} initialized", boards::bsp::BSP::NAME);

    loop {
        devices
            .terminal
            .draw(|frame| {
                let [top, bottom] = Layout::new(
                    embassy_boards::ratatui::layout::Direction::Vertical,
                    [Constraint::Length(4), Constraint::Fill(1)],
                )
                .areas(frame.area());

                frame.render_widget(
                    Paragraph::new(vec![
                        format!("Counter: {}", counter).into(),
                        "Hello World".light_green().into(),
                    ])
                    .block(Block::bordered().title(boards::bsp::BSP::NAME.light_magenta())),
                    top,
                );

                frame.render_widget(&logs, bottom);
            })
            .await
            .unwrap();

        counter = counter.wrapping_add(1);

        if counter % 10 == 0 {
            tracing::info!("Counter {}", counter);
        }

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
                    if let Some(touch) = touches[i] {
                        tracing::info!("Touch x={} y={}", touch.x, touch.y)
                    }
                }
            }
            Err(e) => defmt::error!("Touch error: {}", e),
        }
    }
}
