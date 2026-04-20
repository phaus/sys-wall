use crate::{Module, ModuleCapability, WidgetSize, SystemContext};
use crossterm::event::{Event, KeyCode};

/// The Dashboard module serves as the default landing page.
pub struct DashboardModule {
    pub title: String,
}

impl DashboardModule {
    pub fn new() -> Self {
        Self {
            title: "sys-wall".to_string(),
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
        &self.title
    }

    fn keybinding(&self) -> Option<KeyCode> {
        Some(KeyCode::F(1))
    }

    fn capability(&self) -> ModuleCapability {
        ModuleCapability::WidgetAndPage
    }

    fn widget_size(&self) -> WidgetSize {
        WidgetSize::Small
    }

    fn update(&mut self, _ctx: &SystemContext) -> Result<(), Box<dyn std::error::Error>> {
        Ok(())
    }

    fn render_widget(&self, frame: &mut ratatui::Frame<'_>, area: ratatui::layout::Rect) {
        use ratatui::prelude::Color;

        let text = ratatui::text::Line::from(ratatui::text::Span::styled(
            format!(" {} ", self.title),
            ratatui::style::Style::default().fg(Color::Magenta),
        ));
        let block = ratatui::widgets::Block::default()
            .title(text)
            .border_type(ratatui::widgets::BorderType::Plain)
            .border_style(ratatui::style::Style::default().fg(ratatui::style::Color::Gray));
        frame.render_widget(block, area);
    }

    fn render_page(&self, frame: &mut ratatui::Frame<'_>, area: ratatui::layout::Rect) {
        use ratatui::prelude::{Stylize, Color, Alignment};

        let text = ratatui::text::Line::from(ratatui::text::Span::styled(
            " sys-wall - System Dashboard ".to_string(),
            ratatui::style::Style::default()
                .fg(Color::Green)
                .bold(),
        ));
        let block = ratatui::widgets::Block::default()
            .title_bottom(text)
            .border_type(ratatui::widgets::BorderType::Rounded)
            .title_alignment(Alignment::Center);
        frame.render_widget(block, area);
    }

    fn handle_input(&mut self, _event: &Event) -> Result<bool, Box<dyn std::error::Error>> {
        Ok(false)
    }
}
