use crate::{Module, ModuleCapability, WidgetSize, SystemContext};
use crossterm::event::{Event, KeyCode};
use ratatui::prelude::{Color, Line, Margin, Rect, Span, Style, Text};
use ratatui::widgets::{Block, BorderType, Borders};
use std::time::Duration;

/// System status module - displays hostname, version, uptime, users, and kernel.
pub struct SystemStatusModule {
    hostname: String,
    version: String,
    uptime: String,
    users: String,
    kernel: String,
}

pub fn format_duration(d: Duration) -> String {
    let secs = d.as_secs();
    let days = secs / 86400;
    let hours = (secs % 86400) / 3600;
    let mins = (secs % 3600) / 60;
    format!("{d}d {h}h {m}m", d = days, h = hours, m = mins)
}

impl SystemStatusModule {
    pub fn new() -> Self {
        Self {
            hostname: "unknown".to_string(),
            version: "v0.1.0".to_string(),
            uptime: "0d 0h 0m".to_string(),
            users: "0".to_string(),
            kernel: "unknown".to_string(),
        }
    }
}

impl Default for SystemStatusModule {
    fn default() -> Self {
        Self::new()
    }
}

impl Module for SystemStatusModule {
    fn name(&self) -> &str {
        "system_status"
    }

    fn keybinding(&self) -> Option<KeyCode> {
        None
    }

    fn capability(&self) -> ModuleCapability {
        ModuleCapability::WidgetOnly
    }

    fn widget_size(&self) -> WidgetSize {
        WidgetSize::Small
    }

    fn update(&mut self, ctx: &SystemContext) -> Result<(), Box<dyn std::error::Error>> {
        self.hostname = ctx.hostname.clone();
        self.version = "v0.1.0".to_string();
        self.uptime = format_duration(ctx.uptime);
        self.users = count_users().to_string();
        self.kernel = ctx.kernel_version.clone();
        Ok(())
    }

    fn render_widget(&self, frame: &mut ratatui::Frame<'_>, area: Rect) {
        let text = Text::from(vec![
            Line::from(vec![
                Span::styled(" hostname ", Style::default().fg(Color::Cyan)),
                Span::styled(self.hostname.as_str(), Style::default().fg(Color::White)),
            ]),
            Line::from(vec![
                Span::styled(" version ", Style::default().fg(Color::Cyan)),
                Span::styled(self.version.as_str(), Style::default().fg(Color::White)),
            ]),
            Line::from(vec![
                Span::styled(" uptime ", Style::default().fg(Color::Cyan)),
                Span::styled(self.uptime.as_str(), Style::default().fg(Color::White)),
            ]),
            Line::from(vec![
                Span::styled(" users ", Style::default().fg(Color::Cyan)),
                Span::styled(self.users.as_str(), Style::default().fg(Color::White)),
            ]),
            Line::from(vec![
                Span::styled(" kernel ", Style::default().fg(Color::Cyan)),
                Span::styled(self.kernel.as_str(), Style::default().fg(Color::White)),
            ]),
        ]);

        let block = Block::default()
            .title(format!("─ {} ──", "System"))
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Color::Gray));

        frame.render_widget(block, area);
        frame.render_widget(text, area.inner(Margin { vertical: 1, horizontal: 1 }));
    }

    fn render_page(&self, _frame: &mut ratatui::Frame<'_>, _area: Rect) {
        // WidgetOnly - no page rendering needed.
    }

    fn handle_input(&mut self, _event: &Event) -> Result<bool, Box<dyn std::error::Error>> {
        Ok(false)
    }
}

fn count_users() -> usize {
    0
}
