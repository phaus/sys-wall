use crate::{Module, ModuleCapability, WidgetSize, SystemContext};
use crossterm::event::{Event, KeyCode};
use ratatui::prelude::{
    Color, Line, Margin, Span, Style, Text,
};
use ratatui::widgets::{Block, BorderType, Borders};

/// System Info module — renders a 3-column page: System | Info | Network.
pub struct SystemInfoModule {
    hostname: String,
    uptime: String,
    primary_mac: String,
    system_id: String,
    cpu_usage: String,
    cpu_cores: u16,
    ram_total: String,
    ram_used: String,
    ram_pct: String,
    process_count: u32,
    kernel: String,
    os: String,

    ips: Vec<String>,
    ipv4_gateway: String,
    ipv6_gateway: String,
    dns_servers: Vec<String>,
    tty_path: String,
}

impl SystemInfoModule {
    pub fn new() -> Self {
        Self {
            hostname: "unknown".to_string(),
            uptime: "0d 0h 0m".to_string(),
            primary_mac: "n/a".to_string(),
            system_id: "n/a".to_string(),
            cpu_usage: "0.0%".to_string(),
            cpu_cores: 0,
            ram_total: "0 MB".to_string(),
            ram_used: "0 MB".to_string(),
            ram_pct: "0.0%".to_string(),
            process_count: 0,
            kernel: "unknown".to_string(),
            os: "unknown".to_string(),
            ips: Vec::new(),
            ipv4_gateway: String::new(),
            ipv6_gateway: String::new(),
            dns_servers: Vec::new(),
            tty_path: "/dev/tty1".to_string(),
        }
    }
}

impl Default for SystemInfoModule {
    fn default() -> Self {
        Self::new()
    }
}

impl Module for SystemInfoModule {
    fn name(&self) -> &str {
        "System Info"
    }

    fn keybinding(&self) -> Option<KeyCode> {
        Some(KeyCode::Char('2'))
    }

    fn capability(&self) -> ModuleCapability {
        ModuleCapability::WidgetAndPage
    }

    fn widget_size(&self) -> WidgetSize {
        WidgetSize::Large
    }

    fn update(&mut self, ctx: &SystemContext) -> Result<(), Box<dyn std::error::Error>> {
        self.hostname = ctx.hostname.clone();
        self.uptime = super::super::format_duration(ctx.uptime);
        self.primary_mac = ctx.primary_mac.clone();
        self.system_id = ctx.config.system_id.clone();
        self.cpu_usage = format!("{:.1}%", ctx.cpu_usage);
        self.cpu_cores = ctx.cpu_per_core.len() as u16;
        self.ram_total = format_memory(ctx.memory_total);
        self.ram_used = format_memory(ctx.memory_used);
        let pct = if ctx.memory_total > 0 {
            ctx.memory_used as f64 / ctx.memory_total as f64 * 100.0
        } else {
            0.0
        };
        self.ram_pct = format!("{:.1}%", pct);
        self.process_count = ctx.process_count;
        self.kernel = ctx.kernel_version.clone();
        let mut os_display = ctx.os_name.clone();
        if !ctx.os_codename.is_empty() {
            os_display = format!("{} {}", os_display, ctx.os_codename);
        }
        self.os = os_display;
        // Network data
        self.ips = ctx.ip_addresses.clone();
        self.ipv4_gateway = ctx.ipv4_gateway.clone();
        self.ipv6_gateway = ctx.ipv6_gateway.clone();
        self.dns_servers = ctx.dns_servers.clone();
        self.tty_path = ctx.tty_path.clone();
        Ok(())
    }

    fn render_widget(&self, frame: &mut ratatui::Frame<'_>, area: ratatui::layout::Rect) {
        let mut lines = Vec::new();
        lines.push(Line::from(vec![
            Span::styled(" host ", Style::default().fg(Color::Cyan).bold()),
            Span::styled(self.hostname.as_str(), Style::default().fg(Color::White)),
        ]));
        lines.push(Line::from(vec![
            Span::styled(" CPU  ", Style::default().fg(Color::Red).bold()),
            Span::styled(self.cpu_usage.as_str(), Style::default().fg(Color::White)),
            Span::raw("  |  "),
            Span::styled("cores ", Style::default().fg(Color::Gray)),
            Span::styled(
                self.cpu_cores.to_string(),
                Style::default().fg(Color::White),
            ),
        ]));
        lines.push(Line::from(vec![
            Span::styled(" MEM ", Style::default().fg(Color::Blue).bold()),
            Span::styled(self.ram_used.as_str(), Style::default().fg(Color::White)),
            Span::raw("/"),
            Span::styled(
                self.ram_total.as_str(),
                Style::default().fg(Color::Gray),
            ),
            Span::raw("  "),
            Span::styled(
                self.ram_pct.as_str(),
                Style::default().fg(Color::Cyan),
            ),
        ]));
        lines.push(Line::from(vec![
            Span::styled(" up   ", Style::default().fg(Color::Green).bold()),
            Span::styled(
                self.uptime.as_str(),
                Style::default().fg(Color::White),
            ),
        ]));
        lines.push(Line::from(vec![
            Span::styled(" procs ", Style::default().fg(Color::Green).bold()),
            Span::styled(
                self.process_count.to_string(),
                Style::default().fg(Color::White),
            ),
        ]));
        lines.push(Line::from(vec![
            Span::styled(" kernel ", Style::default().fg(Color::Cyan).bold()),
            Span::styled(
                self.kernel.as_str(),
                Style::default().fg(Color::White),
            ),
        ]));
        lines.push(Line::from(vec![
            Span::styled(" sysid ", Style::default().fg(Color::Yellow).bold()),
            Span::styled(
                self.system_id.as_str(),
                Style::default().fg(Color::White).dim(),
            ),
        ]));
        let text = ratatui::prelude::Text::from(lines);
        let block = Block::default()
            .title(format!(" {} ", "System Info"))
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

    fn render_page(&self, frame: &mut ratatui::Frame<'_>, area: ratatui::layout::Rect) {
        use ratatui::layout::{Constraint, Direction, Layout};

        // Two columns: System Info (left) | Network (right)
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Ratio(1, 3),
                Constraint::Ratio(2, 3),
            ])
            .split(area);

        // Left column: all system data (merged from former Info + System widgets)
        let all_lines: Vec<Line<'_>> = vec![
            Line::from(vec![
                Span::styled(
                    " hostname ",
                    Style::default().fg(Color::Cyan).bold(),
                ),
                Span::styled(
                    self.hostname.as_str(),
                    Style::default().fg(Color::White),
                ),
            ]),
            Line::from(vec![
                Span::styled(
                    " version ",
                    Style::default().fg(Color::Cyan).bold(),
                ),
                Span::styled(
                    "v0.1.0",
                    Style::default().fg(Color::White),
                ),
            ]),
            Line::from(vec![
                Span::styled(
                    " uptime ",
                    Style::default().fg(Color::Cyan).bold(),
                ),
                Span::styled(
                    self.uptime.as_str(),
                    Style::default().fg(Color::White),
                ),
            ]),
            Line::from(vec![
                Span::styled(
                    " OS     ",
                    Style::default().fg(Color::Cyan).bold(),
                ),
                Span::styled(
                    self.os.as_str(),
                    Style::default().fg(Color::White),
                ),
            ]),
            Line::from(vec![
                Span::styled(
                    " kernel ",
                    Style::default().fg(Color::Cyan).bold(),
                ),
                Span::styled(
                    self.kernel.as_str(),
                    Style::default().fg(Color::White),
                ),
            ]),
            Line::from(vec![
                Span::styled(
                    " mac    ",
                    Style::default().fg(Color::Cyan).bold(),
                ),
                Span::styled(
                    self.primary_mac.as_str(),
                    Style::default().fg(Color::White),
                ),
            ]),
            Line::from(vec![
                Span::styled(
                    " CPU    ",
                    Style::default().fg(Color::Cyan).bold(),
                ),
                Span::styled(
                    self.cpu_usage.as_str(),
                    Style::default().fg(Color::White),
                ),
                Span::raw("  |  cores "),
                Span::styled(
                    self.cpu_cores.to_string(),
                    Style::default().fg(Color::White),
                ),
            ]),
            Line::from(vec![
                Span::styled(
                    " Memory ",
                    Style::default().fg(Color::Cyan).bold(),
                ),
                Span::raw(self.ram_used.as_str()),
                Span::raw("/"),
                Span::raw(self.ram_total.as_str()),
                Span::raw("  ("),
                Span::styled(
                    self.ram_pct.as_str(),
                    Style::default().fg(Color::Cyan),
                ),
                Span::raw(")"),
            ]),
            Line::from(vec![
                Span::styled(
                    " Processes ",
                    Style::default().fg(Color::Cyan).bold(),
                ),
                Span::styled(
                    self.process_count.to_string(),
                    Style::default().fg(Color::White),
                ),
            ]),
            Line::from(vec![
                Span::styled(
                    " sysid  ",
                    Style::default().fg(Color::Yellow).bold(),
                ),
                Span::styled(
                    self.system_id.as_str(),
                    Style::default().fg(Color::White).dim(),
                ),
            ]),
        ];

        // Right column: Network data
        let mut network_lines: Vec<Line<'_>> = Vec::new();
        if self.ips.is_empty() {
            network_lines.push(Line::from(vec![
                Span::styled(" ip    ", Style::default().fg(Color::Magenta).bold()),
                Span::styled("n/a", Style::default().fg(Color::Gray)),
            ]));
        } else {
            for ip in &self.ips {
                network_lines.push(Line::from(vec![
                    Span::styled(" ip    ", Style::default().fg(Color::Magenta).bold()),
                    Span::styled(ip, Style::default().fg(Color::White)),
                ]));
            }
        }
        network_lines.push(Line::from(vec![
            Span::styled(" v4gw  ", Style::default().fg(Color::Magenta).bold()),
            if self.ipv4_gateway.is_empty() {
                Span::styled("n/a", Style::default().fg(Color::Gray))
            } else {
                Span::styled(
                    self.ipv4_gateway.as_str(),
                    Style::default().fg(Color::White),
                )
            },
        ]));
        network_lines.push(Line::from(vec![
            Span::styled(" v6gw  ", Style::default().fg(Color::Magenta).bold()),
            if self.ipv6_gateway.is_empty() {
                Span::styled("n/a", Style::default().fg(Color::Gray))
            } else {
                Span::styled(
                    self.ipv6_gateway.as_str(),
                    Style::default().fg(Color::White),
                )
            },
        ]));
        if self.dns_servers.is_empty() {
            network_lines.push(Line::from(vec![
                Span::styled(" dns   ", Style::default().fg(Color::Magenta).bold()),
                Span::styled("n/a", Style::default().fg(Color::Gray)),
            ]));
        } else {
            for dns in &self.dns_servers {
                network_lines.push(Line::from(vec![
                    Span::styled(" dns   ", Style::default().fg(Color::Magenta).bold()),
                    Span::styled(dns, Style::default().fg(Color::White)),
                ]));
            }
        }

        // Render 2 columns
        frame.render_widget(
            Block::default()
                .title(format!(" {} ", "System Info"))
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(Color::Yellow)),
            chunks[0],
        );
        frame.render_widget(Text::from(all_lines), chunks[0].inner(Margin::default()));

        frame.render_widget(
            Block::default()
                .title(format!(" {} ", "Network"))
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(Color::Yellow)),
            chunks[1],
        );
        frame.render_widget(Text::from(network_lines), chunks[1].inner(Margin::default()));
    }

    fn handle_input(&mut self, _event: &Event) -> Result<bool, Box<dyn std::error::Error>> {
        Ok(false)
    }
}

fn format_memory(bytes: u64) -> String {
    format!("{:.0} MB", bytes as f64 / 1_048_576.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_memory_zero() {
        assert_eq!(format_memory(0), "0 MB");
    }

    #[test]
    fn format_memory_one_mb() {
        assert_eq!(format_memory(1_048_576), "1 MB");
    }

    #[test]
    fn format_memory_one_gb() {
        assert_eq!(format_memory(1_073_741_824), "1024 MB");
    }

    #[test]
    fn format_memory_small_values() {
        assert_eq!(format_memory(1000), "0 MB");
        assert_eq!(format_memory(500_000), "0 MB");
        assert_eq!(format_memory(786_432), "1 MB");
    }

    #[test]
    fn format_memory_exact_mb() {
        assert_eq!(format_memory(2 * 1_048_576), "2 MB");
        assert_eq!(format_memory(100 * 1_048_576), "100 MB");
    }

    #[test]
    fn format_memory_large() {
        let two_gb = 2 * 1_073_741_824u64;
        assert_eq!(format_memory(two_gb), "2048 MB");
    }
}
