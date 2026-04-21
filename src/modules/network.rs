use crate::{Module, ModuleCapability, WidgetSize, SystemContext};
use crossterm::event::{Event, KeyCode};
use ratatui::prelude::{Color, Line, Margin, Span, Style, Text};
use ratatui::widgets::{Block, BorderType, Borders};

/// Network module — shows IPs one per line, default gateways, DNS servers.
pub struct NetworkModule {
    ips: Vec<String>,
    ipv4_gateway: String,
    ipv6_gateway: String,
    dns_servers: Vec<String>,
}

impl NetworkModule {
    pub fn new() -> Self {
        Self {
            ips: Vec::new(),
            ipv4_gateway: String::new(),
            ipv6_gateway: String::new(),
            dns_servers: Vec::new(),
        }
    }
}

impl Default for NetworkModule {
    fn default() -> Self {
        Self::new()
    }
}

impl Module for NetworkModule {
    fn name(&self) -> &str {
        "Network"
    }

    fn keybinding(&self) -> Option<KeyCode> {
        None
    }

    fn capability(&self) -> ModuleCapability {
        ModuleCapability::WidgetOnly
    }

    fn widget_size(&self) -> WidgetSize {
        WidgetSize::Large
    }

    fn widget_height(&self) -> u16 {
        22
    }

    fn update(&mut self, ctx: &SystemContext) -> Result<(), Box<dyn std::error::Error>> {
        self.ips = ctx.ip_addresses.clone();
        self.ipv4_gateway = ctx.ipv4_gateway.clone();
        self.ipv6_gateway = ctx.ipv6_gateway.clone();
        self.dns_servers = ctx.dns_servers.clone();
        Ok(())
    }

    fn render_widget(&self, frame: &mut ratatui::Frame<'_>, area: ratatui::layout::Rect) {
        let mut lines = Vec::new();

        // IPs
        if self.ips.is_empty() {
            lines.push(Line::from(vec![
                Span::styled(" ip  ", Style::default().fg(Color::Magenta).bold()),
                Span::styled("n/a", Style::default().fg(Color::Gray)),
            ]));
        } else {
            for ip in &self.ips {
                lines.push(Line::from(vec![
                    Span::styled(" ip  ", Style::default().fg(Color::Magenta).bold()),
                    Span::styled(ip, Style::default().fg(Color::White)),
                ]));
            }
        }

        // Gateways
        lines.push(Line::from(vec![
            Span::styled(" v4gw ", Style::default().fg(Color::Magenta).bold()),
            if self.ipv4_gateway.is_empty() {
                Span::styled("n/a", Style::default().fg(Color::Gray))
            } else {
                Span::styled(self.ipv4_gateway.as_str(), Style::default().fg(Color::White))
            },
        ]));
        lines.push(Line::from(vec![
            Span::styled(" v6gw ", Style::default().fg(Color::Magenta).bold()),
            if self.ipv6_gateway.is_empty() {
                Span::styled("n/a", Style::default().fg(Color::Gray))
            } else {
                Span::styled(self.ipv6_gateway.as_str(), Style::default().fg(Color::White))
            },
        ]));

        // DNS
        if self.dns_servers.is_empty() {
            lines.push(Line::from(vec![
                Span::styled(" dns  ", Style::default().fg(Color::Magenta).bold()),
                Span::styled("n/a", Style::default().fg(Color::Gray)),
            ]));
        } else {
            for dns in &self.dns_servers {
                lines.push(Line::from(vec![
                    Span::styled(" dns  ", Style::default().fg(Color::Magenta).bold()),
                    Span::styled(dns, Style::default().fg(Color::White)),
                ]));
            }
        }

        let text = Text::from(lines);
        let block = Block::default()
            .title(format!(" {} ", "Network"))
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Color::Yellow));
        frame.render_widget(block, area);
        frame.render_widget(
            text,
            area.inner(Margin {
                vertical: 1,
                horizontal: 1,
            }),
        );
    }

    fn render_page(&self, _frame: &mut ratatui::Frame<'_>, _area: ratatui::layout::Rect) {
        // WidgetOnly — no dedicated page.
    }

    fn handle_input(&mut self, _event: &Event) -> Result<bool, Box<dyn std::error::Error>> {
        Ok(false)
    }
}
