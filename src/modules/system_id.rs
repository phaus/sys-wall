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

    fn render_qr_compact_lines(&self, max_cols: u16, max_rows: u16) -> Vec<String> {
        if self.qr_url.is_empty() || max_rows == 0 || max_cols == 0 {
            return Vec::new();
        }

        let qr = QrCode::new(self.qr_url.as_bytes()).unwrap_or_else(|_| {
            QrCode::new(b"syswall").unwrap()
        });
        let colors = qr.to_colors();
        let w = qr.width() as usize;
        if w == 0 {
            return Vec::new();
        }

        // Terminal chars are ~2:1 height:width.
        // Each output row reads 2 QR rows (OR = darker wins).
        // Scale down: if QR is wider than output area, use col_step > 1.
        let total = max_cols.max(max_rows) as usize;
        let step = (w.saturating_sub(1) / total.saturating_sub(1)).max(1);
        let max_cols = max_cols as usize;
        let max_rows = max_rows as usize;
        let w = w as usize;
        let mut result = Vec::new();

        for row in 0..max_rows {
            let mut line = String::with_capacity(max_cols);
            let qr_row_start = row.saturating_mul(step * 2);
            for col in 0..max_cols {
                let qc = col * step;
                let idx_top = qr_row_start.saturating_mul(w).saturating_add(qc);
                let idx_bot = qr_row_start.saturating_mul(w).saturating_add(w).saturating_add(qc);
                let top_dark = idx_top < colors.len()
                    && colors[idx_top] == qrcode::types::Color::Dark;
                let bot_dark = idx_bot < colors.len()
                    && colors[idx_bot] == qrcode::types::Color::Dark;
                line.push(if top_dark || bot_dark { '█' } else { '░' });
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

        // Compute the size for a square QR code.
        // Terminal chars are ~2:1 height:width, so we use equal rows and columns
        // (Dense1x2 style) which makes each output row cover 2 QR grid rows.
        // Reserve 1 row for ID+URL text at the bottom.
        let qr_size = inner.height.saturating_sub(2);
        let qr_width = inner.width.saturating_sub(1);
        let qr_dim = qr_size.min(qr_width);

        if qr_dim >= 12 {
            let qr_lines = self.render_qr_compact_lines(qr_dim, qr_dim);
            if !qr_lines.is_empty() {
                for qr_line in qr_lines {
                    lines.push(Line::raw(qr_line));
                }
                lines.push(Line::raw(""));
            }
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

        // Left: QR code rendered as string blocks
        let block = Block::default()
            .title(" QR Code ")
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Color::Magenta));
        frame.render_widget(block, chunks[0]);

        let inner = chunks[0].inner(Margin {
            vertical: 1,
            horizontal: 1,
        });
        if inner.height > 1 && !self.qr_string.is_empty() {
            // Parse the QR string into lines (it's already a multi-line string)
            let qr_lines: Vec<Line<'_>> = self.qr_string
                .lines()
                .take(inner.height as usize)
                .map(|line| Line::raw(line))
                .collect();
            let text = Text::from(qr_lines);
            frame.render_widget(text, inner);
        } else if inner.height > 1 {
            let text = Text::from(vec![Line::raw(" No data yet")]);
            frame.render_widget(text, inner);
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
        // Verify we can decode the payload
        // (base64url encoding may modify the JSON slightly)
        let encoded = URL_SAFE_NO_PAD.encode(format!(r#"{{"system_id":"a/b+c=d","fingerprint":"fingerprint|data"}}"#).as_bytes());
        assert_eq!(encoded, _url.split("register=").nth(1).unwrap());
    }
}
