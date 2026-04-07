#![no_std]
#![no_main]

extern crate alloc;

use alloc::format;
use alloc::sync::Arc;
use alloc::vec;
use boards::BoardConfig;
use boards::bsp::BSP;
use boards::cortex_m_rt;
use boards::drivers::BoardDrivers;
use boards::drivers::touch::TouchPoint;
use boards::ratatui::widgets::Paragraph;
use common::LogLevel;
use common::Message;
use embassy_boards as boards;
use embassy_boards::display::log_widget::LogWidget;
use embassy_boards::embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_boards::embassy_sync::mutex::Mutex;
use embassy_boards::embassy_time::Instant;
use embassy_boards::ratatui;
use embassy_boards::ratatui::layout::Constraint;
use embassy_boards::ratatui::layout::Layout;
use embassy_boards::ratatui::style::Stylize;
use embassy_boards::ratatui::text::Line;
use embassy_boards::ratatui::widgets::Block;
use embassy_executor::Spawner;
use tracing::Level;

#[derive(Default, Copy, Clone)]
struct State {
    /// Number of messages received from CM4
    cm4_rx: usize,
}

type SharedState = Arc<Mutex<CriticalSectionRawMutex, State>>;

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let mut devices = BSP::<Message>::init().await;

    // Create a tracing log widget
    let logs = LogWidget::default();
    tracing::subscriber::set_global_default(logs.clone()).unwrap();

    tracing::info!("{} initialized", BSP::<Message>::NAME);

    let state = Arc::new(Mutex::<CriticalSectionRawMutex, State>::new(
        State::default(),
    ));

    spawner.spawn(touch_task(devices.touch).unwrap());
    spawner.spawn(message_task(devices.receiver, state.clone()).unwrap());

    let mut counter = 0u32;
    loop {
        let heap_used = BSP::<Message>::heap_used() as f32 / BSP::<Message>::heap_size() as f32;

        let data = {
            let guard = state.lock().await;
            let data = *guard;
            drop(guard);
            data
        };

        devices
            .terminal
            .draw(|frame| {
                let [top, bottom] = Layout::new(
                    embassy_boards::ratatui::layout::Direction::Vertical,
                    [Constraint::Length(4), Constraint::Fill(1)],
                )
                .areas(frame.area());

                let block = Block::bordered().title(" Embassy STM32 ".light_magenta());
                let heap_area = block.inner(top);
                frame.render_widget(block, top);

                let [status, gauge] = Layout::new(
                    embassy_boards::ratatui::layout::Direction::Vertical,
                    [Constraint::Length(1), Constraint::Length(1)],
                )
                .areas(heap_area);

                frame.render_widget(
                    Paragraph::new(Line::from(vec![
                        "Uptime: ".yellow().into(),
                        format!("{} ms", Instant::now().as_millis()).green().into(),
                        " CM4 RX: ".yellow().into(),
                        format!("{}", data.cm4_rx).into(),
                    ])),
                    status,
                );

                frame.render_widget(
                    ratatui::widgets::Gauge::default()
                        .ratio(heap_used as f64)
                        .label(format!(
                            "Heap {}/{} KiB",
                            BSP::<Message>::heap_used() / 1024,
                            BSP::<Message>::heap_size() / 1024
                        ))
                        .light_cyan(),
                    gauge,
                );

                // Render the tracing subscriber widget
                frame.render_widget(&logs, bottom);
            })
            .await
            .unwrap();

        counter = counter.wrapping_add(1);

        devices.dsi.wait_refresh().await;
    }
}

#[embassy_executor::task]
async fn message_task(mut receiver: <BSP<Message> as BoardDrivers>::Receiver, state: SharedState) {
    loop {
        if let Some(msg) = receiver.recv().await {
            state.lock().await.cm4_rx += 1;
            match msg {
                Message::Log { level, message } => match level {
                    LogLevel::Info => tracing::info!("{}", message),
                    LogLevel::Warn => tracing::warn!("{}", message),
                },
                _ => {}
            }
        } else {
            tracing::warn!("receiver returned none")
        }
    }
}

#[embassy_executor::task]
async fn touch_task(mut touch: <BSP<Message> as BoardDrivers>::Touch) {
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
