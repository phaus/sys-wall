use crate::{Module, ModuleCapability, WidgetSize, SystemContext};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use crossterm::event::{Event, KeyCode};
use qrcode::QrCode;
use qrcode::render::Renderer;
use qrcode::render::unicode::Dense1x2;
use ratatui::prelude::{
    Color, Line, Margin, Span, Style, Text,
};
use ratatui::widgets::{Block, BorderType, Borders};

/// System ID module — renders a QR code with device registration payload.
/// The QR encodes: <system_url>/register?<base64url-json>
/// where the JSON contains system_id and the hardware fingerprint.
pub struct SystemIdModule {
    system_id: String,
    qr_url: String,
    fingerprint: String,
    qr_string: String,
}

impl SystemIdModule {
    pub fn new() -> Self {
        Self {
            system_id: String::new(),
            qr_url: String::new(),
            fingerprint: String::new(),
            qr_string: String::new(),
        }
    }

    pub fn generate_payload(system_id: &str, fingerprint: &str, system_url: &str) -> (String, String) {
        let json_str = format!(r#"{{"system_id":"{}","fingerprint":"{}"}}"#, system_id, fingerprint);
        let encoded = URL_SAFE_NO_PAD.encode(json_str.as_bytes());
        let url = format!("{}?register={}", system_url, encoded);
        (json_str, url)
    }

    fn render_qr_string(&self) -> String {
        if self.qr_url.is_empty() {
            return String::new();
        }

        let qr = QrCode::new(self.qr_url.as_bytes()).unwrap_or_else(|_| {
            QrCode::new(b"syswall").unwrap()
        });

        let colors = qr.to_colors();
        let width = qr.width() as usize;

        let mut renderer = Renderer::new(&colors, width, 0);
        renderer
            .dark_color(Dense1x2::Dark)
            .light_color(Dense1x2::Light)
            .module_dimensions(1, 1)
            .build()
    }

    /// Render the QR code into a square compact string.
    ///
    /// Directly samples the QR matrix with 2:1 row and column pairing
    /// to compensate for 2:1 terminal character aspect ratio.
    fn render_qr_compact_lines(&self, target_cols: u16, target_rows: u16) -> Vec<String> {
        if self.qr_url.is_empty() || target_rows == 0 || target_cols == 0 {
            return Vec::new();
        }

        let qr = QrCode::new(self.qr_url.as_bytes()).unwrap_or_else(|_| {
            QrCode::new(b"syswall").unwrap()
        });
        let colors = qr.to_colors();
        let w = qr.width() as usize;
        if w == 0 || colors.is_empty() {
            return Vec::new();
        }

        // Reserve 2 rows: one blank line + one for ID+URL text.
        let qr_height = (target_rows as usize).saturating_sub(2);
        let qr_width = (target_cols as usize).max(8);

        if qr_height < 4 || qr_width < 4 {
            return Vec::new();
        }

        // Each output cell = 2 matrix rows × 2 matrix cols.
        // Terminal chars are ~2:1 tall:wide, so 2×2 pairing = square output.
        // Compute the output size for a visually square QR.
        let max_rows = qr_height.min(w / 2);
        let max_cols = qr_width.min(w / 2);
        let size = max_rows.min(max_cols);

        if size < 4 {
            return Vec::new();
        }

        let mut result = Vec::with_capacity(size);
        for row in 0..size {
            let mut line = String::with_capacity(size);
            for col in 0..size {
                let mc = col * 2;
                let mr = row * 2;
                // Sample a 2×2 block of the matrix, darker wins
                let cells = [
                    (mr, mc),
                    (mr, mc + 1),
                    (mr + 1, mc),
                    (mr + 1, mc + 1),
                ];
                let any_dark = cells.iter().any(|(r, c)| {
                    let idx = r.wrapping_mul(w).wrapping_add(*c);
                    idx < colors.len() && colors[idx] == qrcode::types::Color::Dark
                });
                line.push(if any_dark { '\u{2588}' } else { '\u{2591}' }); // █ ░
            }
            result.push(line);
        }

        result
    }
}

impl Default for SystemIdModule {
    fn default() -> Self {
        Self::new()
    }
}

impl Module for SystemIdModule {
    fn name(&self) -> &str {
        "System ID"
    }

    fn keybinding(&self) -> Option<KeyCode> {
        Some(KeyCode::Char('4'))
    }

    fn capability(&self) -> ModuleCapability {
        ModuleCapability::WidgetAndPage
    }

    fn widget_size(&self) -> WidgetSize {
        WidgetSize::Small
    }

    fn update(&mut self, ctx: &SystemContext) -> Result<(), Box<dyn std::error::Error>> {
        self.system_id = ctx.config.system_id.clone();
        self.fingerprint = compute_fingerprint();
        let (json_str, url) = Self::generate_payload(
            &self.system_id,
            &self.fingerprint,
            &ctx.system_url,
        );
        self.qr_url = url;
        self.qr_string = self.render_qr_string();

        let _ = json_str; // used for generation, not stored

        Ok(())
    }

    fn render_widget(&self, frame: &mut ratatui::Frame<'_>, area: ratatui::layout::Rect) {
        let block = Block::default()
            .title(" System ID ")
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Color::Magenta));
        frame.render_widget(block, area);

        let inner = area.inner(Margin {
            vertical: 1,
            horizontal: 1,
        });
        if inner.width < 5 || inner.height < 2 {
            return;
        }

        let mut lines: Vec<Line<'_>> = Vec::new();

        // Downsample dense QR output to fit widget dimensions.
        // Dense1x2 chars are square in terminal (each char = 2 QR rows × 1 QR col).
        // Reserve 1 row for ID+URL text at the bottom.
        let qr_lines = self.render_qr_compact_lines(inner.width, inner.height);
        if !qr_lines.is_empty() {
            for qr_line in qr_lines {
                lines.push(Line::raw(qr_line));
            }
            lines.push(Line::raw(""));
        }

        // Show system ID and URL
        let id_display = if self.system_id.len() > 12 {
            format!("{}...", &self.system_id[..12])
        } else {
            self.system_id.clone()
        };
        let url_display = if self.qr_url.len() > 30 {
            format!("{:.27}...", self.qr_url)
        } else {
            self.qr_url.clone()
        };
        lines.push(Line::raw(format!("  {:12}  {}", id_display, url_display)));

        let text = Text::from(lines);
        frame.render_widget(text, inner);
    }

    fn render_page(&self, frame: &mut ratatui::Frame<'_>, area: ratatui::layout::Rect) {
        // Split: QR code on left, details on right
        let chunks = ratatui::layout::Layout::default()
            .direction(ratatui::layout::Direction::Horizontal)
            .constraints([
                ratatui::layout::Constraint::Ratio(2, 3),
                ratatui::layout::Constraint::Ratio(1, 3),
            ])
            .split(area);

        // Left: QR code rendered as string blocks with automatic downsampling
        let block = Block::default()
            .title(" QR Code ")
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Color::Magenta));
        frame.render_widget(block, chunks[0]);

        let inner = chunks[0].inner(Margin {
            vertical: 1,
            horizontal: 1,
        });
        if inner.height > 2 {
            let qr_lines = self.render_qr_compact_lines(inner.width, inner.height);
            if !qr_lines.is_empty() {
                let qr_lines: Vec<Line<'_>> = qr_lines
                    .into_iter()
                    .map(Line::raw)
                    .collect();
                frame.render_widget(Text::from(qr_lines), inner);
            } else {
                let text = Text::from(vec![Line::raw(" No data yet")]);
                frame.render_widget(text, inner);
            }
        }

        // Right: detail panel
        let mut lines: Vec<Line<'_>> = Vec::new();
        lines.push(Line::from(vec![
            Span::styled(" System ID ", Style::default().fg(Color::Yellow).bold()),
        ]));
        lines.push(Line::from(vec![
            Span::raw(self.system_id.as_str()),
        ]));
        lines.push(Line::raw(""));
        lines.push(Line::from(vec![
            Span::styled(" Fingerprint ", Style::default().fg(Color::Cyan).bold()),
        ]));
        lines.push(Line::from(vec![
            Span::raw(self.fingerprint.as_str()),
        ]));
        lines.push(Line::raw(""));
        lines.push(Line::from(vec![
            Span::styled(" Registration URL ", Style::default().fg(Color::Blue).bold()),
        ]));
        lines.push(Line::from(vec![
            Span::raw(self.qr_url.as_str()),
        ]));

        let detail_block = Block::default()
            .title(" Details ")
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Color::Magenta));
        frame.render_widget(detail_block, chunks[1]);
        frame.render_widget(
            Text::from(lines),
            chunks[1].inner(Margin {
                vertical: 1,
                horizontal: 1,
            }),
        );
    }

    fn handle_input(&mut self, _event: &Event) -> Result<bool, Box<dyn std::error::Error>> {
        Ok(false)
    }
}

fn compute_fingerprint() -> String {
    let hostname = std::fs::read_to_string("/etc/hostname")
        .map(|h| h.trim().to_string())
        .unwrap_or_else(|_| "unknown".to_string());
    let mac = crate::collect_primary_mac();
    let kernel = crate::collect_kernel_version();
    format!("{}|{}|{}", hostname, mac, kernel)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_payload_basic() {
        let (json, url) = SystemIdModule::generate_payload(
            "test-id-123",
            "host|aa:bb:cc|5.10.0",
            "https://debug.consolving.net/system",
        );
        assert!(json.contains("test-id-123"));
        assert!(json.contains("host"));
        assert!(url.contains("register="));
        assert!(url.contains("https://debug.consolving.net/system"));
    }

    #[test]
    fn generate_payload_special_chars_in_id() {
        let (json, _url) = SystemIdModule::generate_payload(
            "a/b+c=d",
            "fingerprint|data",
            "https://example.com/system",
        );
        assert!(json.contains("a/b+c=d"));
        let encoded = URL_SAFE_NO_PAD.encode(format!(r#"{{"system_id":"a/b+c=d","fingerprint":"fingerprint|data"}}"#).as_bytes());
        assert_eq!(encoded, _url.split("register=").nth(1).unwrap());
    }
}
