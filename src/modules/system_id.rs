use crate::{Module, ModuleCapability, WidgetSize, SystemContext};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use crossterm::event::{Event, KeyCode};
use qrcode::{QrCode, EcLevel};
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
    /// Whether the device is already registered (GET check returned {"status":"ok"}).
    registered: bool,
}

impl SystemIdModule {
    pub fn new() -> Self {
        Self {
            system_id: String::new(),
            qr_url: String::new(),
            fingerprint: String::new(),
            registered: false,
        }
    }

    pub fn generate_payload(system_id: &str, fingerprint: &str, system_url: &str) -> (String, String) {
        let json_str = format!(r#"{{"system_id":"{}","fingerprint":"{}"}}"#, system_id, fingerprint);
        let encoded = URL_SAFE_NO_PAD.encode(json_str.as_bytes());
        let url = format!("{}?register={}", system_url, encoded);
        (json_str, url)
    }

    /// Build the GET URL to check registration status.
    fn check_url(system_id: &str, fingerprint: &str, system_url: &str) -> String {
        let json_str = format!(r#"{{"system_id":"{}","fingerprint":"{}"}}"#, system_id, fingerprint);
        let encoded = URL_SAFE_NO_PAD.encode(json_str.as_bytes());
        format!("{}?get={}", system_url, encoded)
    }

    /// Check if the device is registered by querying the server.
    fn check_registered(system_id: &str, fingerprint: &str, system_url: &str) -> bool {
        if system_url.is_empty() || system_id.is_empty() {
            return false;
        }
        let url = Self::check_url(system_id, fingerprint, system_url);
        let resp = ureq::get(&url)
            .timeout(std::time::Duration::from_secs(3))
            .call();
        match resp {
            Ok(resp) => {
                if let Ok(body) = resp.into_string() {
                    // Parse minimal JSON: look for "status":"ok"
                    body.contains(r#""status":"ok"#) || body.contains(r#""status": "ok"#)
                } else {
                    false
                }
            }
            Err(_) => false,
        }
    }

    /// Render the QR code as styled Lines using Unicode half-block characters.
    ///
    /// Each terminal character encodes 2 vertical QR modules using ▀, ▄, █, or space
    /// with black/white fg/bg colors. Each QR module maps to exactly 1 terminal
    /// column (no downscaling), preserving full module fidelity required for scanning.
    ///
    /// Uses EcLevel::L (low error correction) to minimize QR size and fit more data
    /// into smaller terminal areas.
    ///
    /// Each line is padded with white background to fill the widget width,
    /// ensuring the quiet zone renders as white (not the terminal's dark background).
    fn render_qr_lines(url: &str, target_cols: u16, target_rows: u16) -> Vec<Line<'static>> {
        if url.is_empty() || target_rows == 0 || target_cols == 0 {
            return Vec::new();
        }

        let qr = QrCode::with_error_correction_level(url.as_bytes(), EcLevel::L)
            .unwrap_or_else(|_| {
                QrCode::with_error_correction_level(b"syswall", EcLevel::L).unwrap()
            });
        let colors = qr.to_colors();
        let w = qr.width() as usize;
        if w == 0 || colors.is_empty() {
            return Vec::new();
        }

        // Add 2-module quiet zone on each side
        let quiet = 2usize;
        let total_w = w + quiet * 2;
        let total_h = w + quiet * 2;

        let target_c = target_cols as usize;
        let target_r = target_rows as usize;

        // Each output row = 2 QR rows (half-block), each output col = 1 QR module
        let out_cols = total_w;
        let out_rows = (total_h + 1) / 2;

        if out_cols > target_c || out_rows > target_r {
            // QR code doesn't fit — don't render a broken one
            return Vec::new();
        }

        let is_dark = |qr_row: usize, qr_col: usize| -> bool {
            if qr_row < quiet || qr_row >= quiet + w || qr_col < quiet || qr_col >= quiet + w {
                return false; // quiet zone = light
            }
            let r = qr_row - quiet;
            let c = qr_col - quiet;
            colors[r * w + c] == qrcode::types::Color::Dark
        };

        let white = Color::White;
        let black = Color::Black;
        let white_span = |n: usize| -> Span<'static> {
            Span::styled(" ".repeat(n), Style::default().fg(white).bg(white))
        };

        let mut result = Vec::with_capacity(out_rows);
        let mut qr_row = 0usize;
        while qr_row < total_h {
            let has_bot = qr_row + 1 < total_h;

            // Center horizontally with white padding
            let pad_left = (target_c.saturating_sub(out_cols)) / 2;
            let pad_right = target_c.saturating_sub(out_cols).saturating_sub(pad_left);

            let mut spans: Vec<Span<'static>> = Vec::new();
            if pad_left > 0 {
                spans.push(white_span(pad_left));
            }

            for col in 0..total_w {
                let top = is_dark(qr_row, col);
                let bot = if has_bot { is_dark(qr_row + 1, col) } else { false };

                let (ch, fg, bg) = match (top, bot) {
                    (false, false) => (' ', white, white),
                    (true, true)   => (' ', black, black),
                    (true, false)  => ('▀', black, white),
                    (false, true)  => ('▄', black, white),
                };
                spans.push(Span::styled(
                    String::from(ch),
                    Style::default().fg(fg).bg(bg),
                ));
            }

            if pad_right > 0 {
                spans.push(white_span(pad_right));
            }

            result.push(Line::from(spans));
            qr_row += 2;
        }

        // Center vertically with white padding
        let qr_term_rows = result.len();
        if qr_term_rows < target_r {
            let pad_top = (target_r - qr_term_rows) / 2;
            let pad_bot = target_r - qr_term_rows - pad_top;
            let white_line = || Line::from(vec![white_span(target_c)]);
            let mut padded = Vec::with_capacity(target_r);
            for _ in 0..pad_top {
                padded.push(white_line());
            }
            padded.append(&mut result);
            for _ in 0..pad_bot {
                padded.push(white_line());
            }
            return padded;
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
        "Scan to Add"
    }

    fn keybinding(&self) -> Option<KeyCode> {
        Some(KeyCode::Char('1'))
    }

    fn capability(&self) -> ModuleCapability {
        if self.registered {
            ModuleCapability::WidgetOnly
        } else {
            ModuleCapability::WidgetAndPage
        }
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
        let _ = json_str;

        // Check registration status
        self.registered = Self::check_registered(&self.system_id, &self.fingerprint, &ctx.system_url);

        Ok(())
    }

    fn render_widget(&self, frame: &mut ratatui::Frame<'_>, area: ratatui::layout::Rect) {
        let block = Block::default()
            .title(" System ID ")
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Color::Magenta));
        frame.render_widget(block, area);

        let inner = area.inner(Margin { vertical: 1, horizontal: 1 });
        if inner.width < 5 || inner.height < 1 {
            return;
        }

        let mut lines: Vec<Line<'_>> = Vec::new();
        lines.push(Line::from(vec![
            Span::styled(" ID   ", Style::default().fg(Color::Magenta).bold()),
            Span::styled(self.system_id.as_str(), Style::default().fg(Color::White)),
        ]));
        lines.push(Line::from(vec![
            Span::styled(" FP   ", Style::default().fg(Color::Magenta).bold()),
            Span::styled(self.fingerprint.as_str(), Style::default().fg(Color::Gray)),
        ]));
        if inner.height > 2 {
            if self.registered {
                lines.push(Line::from(vec![
                    Span::styled(" ✓ registered ", Style::default().fg(Color::Green).bold()),
                ]));
            } else {
                lines.push(Line::from(vec![
                    Span::styled(" Scan QR → press [1] ", Style::default().fg(Color::DarkGray)),
                ]));
            }
        }

        frame.render_widget(Text::from(lines), inner);
    }

    fn render_page(&self, frame: &mut ratatui::Frame<'_>, area: ratatui::layout::Rect) {
        let block = Block::default()
            .title(" Scan to Add ")
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Color::Magenta));
        frame.render_widget(block, area);

        let inner = area.inner(Margin { vertical: 1, horizontal: 1 });
        if inner.width < 5 || inner.height < 4 {
            return;
        }

        // Reserve 3 rows at bottom for details text
        let qr_rows = inner.height.saturating_sub(3);
        let qr_lines = Self::render_qr_lines(&self.qr_url, inner.width, qr_rows);

        let mut lines: Vec<Line<'_>> = Vec::new();
        if !qr_lines.is_empty() {
            lines.extend(qr_lines);
        } else {
            lines.push(Line::styled(
                " QR code too large for this terminal size ",
                Style::default().fg(Color::Red),
            ));
        }

        lines.push(Line::raw(""));
        lines.push(Line::from(vec![
            Span::styled(" System ID  ", Style::default().fg(Color::Yellow).bold()),
            Span::raw(&self.system_id),
            Span::raw("    "),
            Span::styled(" Fingerprint  ", Style::default().fg(Color::Cyan).bold()),
            Span::raw(&self.fingerprint),
        ]));
        lines.push(Line::from(vec![
            Span::styled(" URL  ", Style::default().fg(Color::Blue).bold()),
            Span::raw(&self.qr_url),
        ]));

        frame.render_widget(Text::from(lines), inner);
    }

    fn handle_input(&mut self, _event: &Event) -> Result<bool, Box<dyn std::error::Error>> {
        Ok(false)
    }
}

fn compute_fingerprint() -> String {
    // Hostname: try /etc/hostname, then `hostname` command
    let hostname = std::fs::read_to_string("/etc/hostname")
        .ok()
        .map(|h| h.trim().to_string())
        .filter(|h| !h.is_empty())
        .or_else(|| {
            std::process::Command::new("hostname")
                .output()
                .ok()
                .filter(|o| o.status.success())
                .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        })
        .unwrap_or_else(|| "unknown".to_string());
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

    /// Convert rendered QR Lines back into a binary pixel grid and decode with rxing.
    /// Each terminal character encodes 2 vertical pixels via half-block chars:
    ///   ' ' with white bg → top=white, bot=white
    ///   ' ' with black bg → top=black, bot=black
    ///   '▀' → top=dark(fg), bot=light(bg)
    ///   '▄' → top=light(bg), bot=dark(fg)
    fn decode_qr_lines(lines: &[Line<'_>]) -> Result<String, String> {
        use rxing::Reader;

        if lines.is_empty() {
            return Err("no lines".into());
        }

        // Determine terminal grid dimensions
        let term_height = lines.len();
        let term_width: usize = lines.iter().map(|l| {
            l.spans.iter().map(|s| s.content.chars().count()).sum::<usize>()
        }).max().unwrap_or(0);

        if term_width == 0 {
            return Err("zero width".into());
        }

        // Each terminal char = 1 pixel wide, 2 pixels tall (half-block encoding)
        // But 1px-per-module is too small for decoders.
        // Upscale each pixel by a factor to give the decoder enough resolution.
        let upscale = 4u32;
        let pixel_width = (term_width as u32) * upscale;
        let pixel_height = (term_height as u32) * 2 * upscale;

        let mut pixels = vec![255u8; (pixel_width * pixel_height) as usize];

        for (row_idx, line) in lines.iter().enumerate() {
            let top_y = (row_idx as u32) * 2;
            let bot_y = top_y + 1;
            let mut x = 0u32;
            for span in &line.spans {
                let fg_dark = matches!(span.style.fg, Some(Color::Black));
                let bg_dark = matches!(span.style.bg, Some(Color::Black));
                for ch in span.content.chars() {
                    if x >= term_width as u32 { break; }
                    let (top_dark, bot_dark) = match ch {
                        '▀' => (fg_dark, bg_dark),
                        '▄' => (bg_dark, fg_dark),
                        ' ' => (bg_dark, bg_dark),
                        _ => (bg_dark, bg_dark),
                    };
                    // Fill upscaled block
                    for dy in 0..upscale {
                        for dx in 0..upscale {
                            let px = x * upscale + dx;
                            let py_top = top_y * upscale + dy;
                            let py_bot = bot_y * upscale + dy;
                            if top_dark {
                                pixels[(py_top * pixel_width + px) as usize] = 0;
                            }
                            if py_bot < pixel_height / upscale * upscale && bot_dark {
                                pixels[(py_bot * pixel_width + px) as usize] = 0;
                            }
                        }
                    }
                    x += 1;
                }
            }
        }

        // Convert to DynamicImage (Luma8)
        let img = image::GrayImage::from_raw(pixel_width, pixel_height, pixels)
            .ok_or_else(|| "failed to create image".to_string())?;
        let dyn_img = image::DynamicImage::ImageLuma8(img);

        let lum = rxing::common::HybridBinarizer::new(
            rxing::BufferedImageLuminanceSource::new(dyn_img),
        );
        let mut bitmap = rxing::BinaryBitmap::new(lum);
        match rxing::qrcode::QRCodeReader::default().decode(&mut bitmap) {
            Ok(result) => Ok(result.getText().to_string()),
            Err(e) => Err(format!("decode failed: {e}")),
        }
    }

    #[test]
    fn qr_render_scale1_decodable() {
        let url = "https://example.com/test";
        let lines = SystemIdModule::render_qr_lines(url, 120, 40);
        assert!(!lines.is_empty(), "render_qr_lines returned empty");
        let decoded = decode_qr_lines(&lines)
            .expect("QR code at scale=1 should be decodable");
        assert_eq!(decoded, url);
    }

    #[test]
    fn qr_render_widget_size_short_url_decodable() {
        // Short URL that fits in widget
        let url = "https://example.com/r?id=abc123";
        let lines = SystemIdModule::render_qr_lines(url, 118, 18);
        assert!(!lines.is_empty(), "short URL QR should fit in widget");
        let decoded = decode_qr_lines(&lines)
            .expect("Short URL QR should be decodable at widget size");
        assert_eq!(decoded, url);
    }

    #[test]
    fn qr_render_widget_size_long_url_graceful() {
        // Long URL that can't fit in widget — should return empty (no broken QR)
        let url = "https://debug.consolving.net/system?register=eyJzeXN0ZW1faWQiOiJiNGVhZjI1YS1lMWQ5LTQwNDgtYWRjMS0xYjc0MjdiZmQ2NjIiLCJmaW5nZXJwcmludCI6InVua25vd258MDI6NDI6YWM6MTI6MDA6MDB8MjUuNC4wIn0";
        let lines = SystemIdModule::render_qr_lines(url, 118, 18);
        if !lines.is_empty() {
            let decoded = decode_qr_lines(&lines)
                .expect("If QR is rendered, it must be decodable");
            assert_eq!(decoded, url);
        }
    }

    #[test]
    fn qr_render_realistic_payload_decodable() {
        let (_, url) = SystemIdModule::generate_payload(
            "b4eaf25a-e1d9-4048-adc1-1b7427bfd662",
            "unknown|02:42:ac:12:00:00|25.4.0",
            "https://debug.consolving.net/system",
        );
        let lines = SystemIdModule::render_qr_lines(&url, 200, 60);
        assert!(!lines.is_empty(), "render_qr_lines returned empty for full page");
        let decoded = decode_qr_lines(&lines)
            .expect("QR code with realistic payload should be decodable at full page size");
        assert_eq!(decoded, url);
    }
}
