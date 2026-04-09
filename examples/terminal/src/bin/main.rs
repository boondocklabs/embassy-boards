#![no_std]
#![no_main]

use boards::BoardConfig;
use boards::bsp::Board;
use boards::cortex_m_rt;
use embassy_boards as boards;
use embassy_boards::embassy_time::Timer;
use embassy_boards::ratatui::layout::{Constraint, Layout};
use embassy_boards::ratatui::style::Stylize;
use embassy_boards::ratatui::text::{Line, Span};
use embassy_boards::ratatui::widgets::{Block, Paragraph, RatatuiLogo};
use embassy_executor::Spawner;

extern crate alloc;

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let mut devices = Board::init().await;

    defmt::info!("{} initialized", Board::NAME);

    let mut count = 0u32;
    loop {
        devices
            .terminal
            .draw(|frame| {
                use alloc::format;
                use alloc::vec;
                use embassy_boards::ratatui::widgets::BorderType;

                let [top, bottom] = Layout::vertical([Constraint::Length(2), Constraint::Fill(1)])
                    .areas(frame.area());

                frame.render_widget(RatatuiLogo::small(), top);

                frame.render_widget(
                    Paragraph::new(vec![
                        Line::from(vec![
                            "Board: ".green(),
                            format!("{}", Board::NAME).yellow().into(),
                        ]),
                        format!("Heap free: {} KiB", Board::heap_free() / 1024).into(),
                        format!("Counter: {}", count).magenta().into(),
                    ])
                    .block(
                        Block::bordered()
                            .border_type(BorderType::Rounded)
                            .title(" Embassy Boards "),
                    ),
                    bottom,
                )
            })
            .await
            .unwrap();
        devices.led.mask(count);
        count = count.wrapping_add(1);
        Timer::after_millis(50).await;
    }
}
