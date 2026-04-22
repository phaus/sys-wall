use crate::{Module, ModuleCapability, WidgetSize, SystemContext};
use crossterm::event::{Event, KeyCode};
use ratatui::prelude::{Color, Line, Margin, Span, Style, Text};
use ratatui::widgets::{Block, BorderType, Borders};

/// The Dashboard module serves as the default landing page.
/// Renders a compact system summary (hostname, MAC) on the widget, with a
/// styled page header when selected.
pub struct DashboardModule {
    hostname: String,
    primary_mac: String,
}

impl DashboardModule {
    pub fn new() -> Self {
        Self {
            hostname: "unknown".to_string(),
            primary_mac: "n/a".to_string(),
        }
    }
}

impl Default for DashboardModule {
    fn default() -> Self {
        Self::new()
    }
}

impl Module for DashboardModule {
    fn name(&self) -> &str {
        "Dashboard"
    }

    fn keybinding(&self) -> Option<KeyCode> {
        Some(KeyCode::Char('2'))
    }

    fn capability(&self) -> ModuleCapability {
        ModuleCapability::WidgetAndPage
    }

    fn widget_size(&self) -> WidgetSize {
        WidgetSize::Small
    }

    fn update(&mut self, ctx: &SystemContext) -> Result<(), Box<dyn std::error::Error>> {
        self.hostname = ctx.hostname.clone();
        self.primary_mac = ctx.primary_mac.clone();
        Ok(())
    }

    fn render_widget(&self, frame: &mut ratatui::Frame<'_>, area: ratatui::layout::Rect) {
        let lines = vec![
            Line::from(vec![
                Span::styled(
                    self.hostname.as_str(),
                    Style::default().fg(Color::Magenta).bold(),
                ),
            ]),
            Line::from(vec![
                Span::styled(
                    "mac  ",
                    Style::default().fg(Color::Cyan).bold(),
                ),
                Span::styled(
                    self.primary_mac.as_str(),
                    Style::default().fg(Color::White),
                ),
            ]),
        ];
        let text = Text::from(lines);
        let block = Block::default()
            .title(format!(" {} ", "sys-wall"))
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Color::Magenta));
        frame.render_widget(block, area);
        frame.render_widget(
            text,
            area.inner(Margin {
                vertical: 0,
                horizontal: 1,
            }),
        );
    }

    fn render_page(&self, frame: &mut ratatui::Frame<'_>, area: ratatui::layout::Rect) {
        use ratatui::prelude::Alignment;

        let block = Block::default()
            .title_top(
                Line::from(vec![
                    Span::raw("   "),
                    Span::styled(
                        " sys-wall ".to_string(),
                        Style::default().fg(Color::Green).bold(),
                    ),
                ])
                .alignment(Alignment::Center),
            )
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Color::Gray));
        frame.render_widget(block, area);
    }

    fn handle_input(
        &mut self,
        _event: &Event,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        Ok(false)
    }
}
