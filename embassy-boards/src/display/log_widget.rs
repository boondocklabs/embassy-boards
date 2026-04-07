//! Ratatui tracing log widget

extern crate alloc;

use alloc::vec;
use alloc::{collections::vec_deque::VecDeque, sync::Arc, vec::Vec};
use embassy_sync::blocking_mutex::{Mutex, raw::CriticalSectionRawMutex};
use ratatui::style::Style;
use ratatui::text;
use ratatui::{
    style::Stylize,
    text::Line,
    widgets::{Block, Borders, Paragraph, Widget, Wrap},
};
use tracing::Level;
use tracing::{
    Subscriber,
    field::{Field, Visit},
};

#[derive(Clone)]
pub struct LogWidget {
    events: Arc<Mutex<CriticalSectionRawMutex, VecDeque<Line<'static>>>>,
}

impl Default for LogWidget {
    fn default() -> Self {
        Self {
            events: Arc::new(Mutex::new(VecDeque::new())),
        }
    }
}

impl Widget for &LogWidget {
    fn render(self, area: ratatui::prelude::Rect, buf: &mut ratatui::prelude::Buffer)
    where
        Self: Sized,
    {
        let block = Block::default().title("Logs").borders(Borders::ALL);
        let inner = block.inner(area);
        block.render(area, buf);

        if inner.width == 0 || inner.height == 0 {
            return;
        }

        self.events.lock(|events| {
            let visible = area.height as usize;
            let start = events.len().saturating_sub(visible);

            let lines: Vec<Line> = events.clone().into_iter().rev().skip(start).collect();

            let paragraph = Paragraph::new(lines)
                .block(
                    Block::default()
                        .title("Logs".light_cyan())
                        .borders(Borders::ALL)
                        .border_type(ratatui::widgets::BorderType::Rounded),
                )
                .wrap(Wrap { trim: false });

            paragraph.render(area, buf);
        });

        unsafe {
            self.events
                .lock_mut(|events| events.truncate(area.height as usize));
        }
    }
}

struct Visitor {
    message: Vec<text::Span<'static>>,
}

impl Visitor {
    fn new(level: &Level, _target: &str) -> Self {
        let style = match *level {
            Level::INFO => Style::new().light_green(),
            Level::WARN => Style::new().light_yellow(),
            Level::DEBUG => Style::new().light_cyan(),
            Level::ERROR => Style::new().light_red(),
            _ => Style::new().white(),
        };
        Self {
            message: vec![
                "[".into(),
                text::Span::from(alloc::format!("{}", level.as_str())).style(style),
                "] ".into(),
            ],
        }
    }
}

impl Visit for Visitor {
    fn record_debug(&mut self, field: &Field, value: &dyn core::fmt::Debug) {
        if field.name() == "message" {
            self.message
                .push(text::Span::from(alloc::format!("{:?}", value)));
        } else {
            self.message.push(text::Span::from(alloc::format!(
                "{}={:?}",
                field.name(),
                value
            )));
        }
    }

    fn record_str(&mut self, field: &Field, value: &str) {
        if field.name() == "message" {
            self.message
                .push(text::Span::from(alloc::format!("{}", value)));
        } else {
            self.message.push(text::Span::from(alloc::format!(
                "{}={}",
                field.name(),
                value
            )));
        }
    }
}

impl Subscriber for LogWidget {
    fn enabled(&self, metadata: &tracing::Metadata<'_>) -> bool {
        true
    }

    fn new_span(&self, span: &tracing::span::Attributes<'_>) -> tracing::span::Id {
        tracing::span::Id::from_u64(1)
    }

    fn record(&self, span: &tracing::span::Id, values: &tracing::span::Record<'_>) {
        defmt::info!("record");
    }

    fn record_follows_from(&self, span: &tracing::span::Id, follows: &tracing::span::Id) {
        defmt::info!("record_follows_from");
    }

    fn event(&self, event: &tracing::Event<'_>) {
        let meta = event.metadata();

        let mut visitor = Visitor::new(meta.level(), meta.target());

        event.record(&mut visitor as &mut dyn tracing::field::Visit);

        unsafe {
            self.events.lock_mut(|guard| {
                guard.push_front(Line::from(visitor.message));
                guard.truncate(512);
            });
        }
    }

    fn enter(&self, span: &tracing::span::Id) {
        defmt::info!("enter span");
    }

    fn exit(&self, span: &tracing::span::Id) {
        defmt::info!("exit span");
    }
}
