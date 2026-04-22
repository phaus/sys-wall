use crate::{Module, ModuleCapability, WidgetSize, SystemContext};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use crossterm::event::{Event, KeyCode};
use framebuffer;
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
    /// Whether we successfully rendered a FB QR code (so we skip ASCII fallback).
    fb_used: bool,
}

impl SystemIdModule {
    pub fn new() -> Self {
        Self {
            system_id: String::new(),
            qr_url: String::new(),
            fingerprint: String::new(),
            registered: false,
            fb_used: false,
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

    fn render_qr_lines(url: &str, target_cols: u16, target_rows: u16) -> Vec<Line<'static>> {
        if url.is_empty() || target_rows == 0 || target_cols == 0 {
            return Vec::new();
        }

        let qr = qrcode::QrCode::with_error_correction_level(url.as_bytes(), qrcode::EcLevel::L)
            .unwrap_or_else(|_| {
                qrcode::QrCode::with_error_correction_level(b"syswall", qrcode::EcLevel::L).unwrap()
            });

        let colors = qr.to_colors();
        let size = qr.width() as usize;

        let needed_cols = size;
        let needed_rows = size;
        if needed_cols > target_cols as usize || needed_rows > target_rows as usize {
            return Vec::new();
        }

        // On macOS (non-Linux), use half-block characters for proper 1:1 QR aspect ratio.
        // Linux uses `#` + space (FB rendering handles pixel-perfect QR separately).
        let is_linux = std::env::var("TERM").unwrap_or_default() == "linux";

        if is_linux {
            let mut result = Vec::new();
            for row in 0..size {
                let mut chars = String::with_capacity(size);
                for col in 0..size {
                    let dark = colors[row * size + col] == qrcode::Color::Dark;
                    chars.push(if dark { '#' } else { ' ' });
                }
                let padding = target_cols as usize - size;
                if padding > 0 {
                    chars.push_str(&" ".repeat(padding));
                }
                result.push(Line::raw(chars));
            }
            result
        } else {
            // macOS / other: half-block characters give square QR (2:1 vertical compensation)
            let bg = Color::White;
            let mut result = Vec::new();

            for row in (0..size).step_by(2) {
                let has_bot = row + 1 < size;
                let mut spans: Vec<Span<'static>> = Vec::new();

                for col in 0..size {
                    let top = colors[row * size + col] == qrcode::Color::Dark;
                    let bot = if has_bot {
                        colors[(row + 1) * size + col] == qrcode::Color::Dark
                    } else {
                        false
                    };
                    let ch = match (top, bot) {
                        (true, true) => '\u{2588}',  // █ dark/dark
                        (true, false) => '\u{2580}', // ▀ dark/light (top dark)
                        (false, true) => '\u{2584}', // ▄ light/dark (bottom dark)
                        (false, false) => ' ',
                    };
                    spans.push(Span::styled(ch.to_string(), Style::default().bg(bg)));
                }

                let padding = target_cols as usize - size;
                if padding > 0 {
                    spans.push(Span::styled(" ".repeat(padding), Style::default().bg(bg)));
                }

                result.push(Line::from(spans));
            }

            result
        }
    }

    /// Render a QR code directly to the framebuffer using pixel-perfect rendering.
    /// 
    /// This opens /dev/fb0, draws the QR code with large pixel blocks
    /// and centers it on screen. No VT mode switching — just direct pixel
    /// writes. Returns true if rendering succeeded.
    fn render_qr_fb_once(&mut self, module_size: u32) -> bool {
        if self.qr_url.is_empty() || self.fb_used {
            return self.fb_used;
        }

        let qr = qrcode::QrCode::with_error_correction_level(self.qr_url.as_bytes(), qrcode::EcLevel::L)
            .unwrap_or_else(|_| {
                qrcode::QrCode::with_error_correction_level(b"syswall", qrcode::EcLevel::L).unwrap()
            });

        let colors = qr.to_colors();
        let size = qr.width() as u32;
        
        let min_pixel_size = size.saturating_sub(2) * module_size;
        if min_pixel_size >= 1280 {
            return false;
        }

        let mut fb = match framebuffer::Framebuffer::new("/dev/fb0") {
            Ok(f) => f,
            Err(_) => return false,
        };

        let width = fb.var_screen_info.xres as u32;
        let height = fb.var_screen_info.yres as u32;
        let line_length = fb.fix_screen_info.line_length as u32;
        let bpp = fb.var_screen_info.bits_per_pixel;
        
        let qr_pixel_width = size * module_size;
        let qr_pixel_height = size * module_size;
        let x_offset = (width / 2) - (qr_pixel_width / 2);
        let y_offset = (height / 2) - (qr_pixel_height / 2);

        let (ok, _bpp) = match bpp {
            32 => {
                for row in 0..size {
                    for col in 0..size {
                        let dark = colors[(row * size + col) as usize] == qrcode::Color::Dark;
                        let module_x = x_offset + col * module_size;
                        let module_y = y_offset + row * module_size;
                        let (r, g, b) = if dark { (0, 0, 0) } else { (255, 255, 255) };
                        for dy in 0..module_size {
                            let py = module_y + dy;
                            if py >= height { break; }
                            for dx in 0..module_size {
                                let px = module_x + dx;
                                if px >= width { break; }
                                let offset = (py as usize) * line_length as usize + (px as usize) * 4;
                                if offset + 4 <= fb.frame.len() {
                                    fb.frame[offset..offset+4].copy_from_slice(&[r, g, b, 255]);
                                }
                            }
                        }
                    }
                }
                (true, true)
            }
            16 => {
                for row in 0..size {
                    for col in 0..size {
                        let dark = colors[(row * size + col) as usize] == qrcode::Color::Dark;
                        let module_x = x_offset + col * module_size;
                        let module_y = y_offset + row * module_size;
                        let (r, g, b) = if dark { (0, 0, 0) } else { (255, 255, 255) };
                        let pixel = (((r >> 3) as u16) << 11) | (((g >> 2) as u16) << 5) | ((b >> 3) as u16);
                        for dy in 0..module_size {
                            let py = module_y + dy;
                            if py >= height { break; }
                            for dx in 0..module_size {
                                let px = module_x + dx;
                                if px >= width { break; }
                                let offset = (py as usize) * line_length as usize + (px as usize) * 2;
                                if offset + 2 <= fb.frame.len() {
                                    fb.frame[offset..offset+2].copy_from_slice(&pixel.to_le_bytes());
                                }
                            }
                        }
                    }
                }
                (true, true)
            }
            _ => (false, false),
        };
        
        if ok {
            self.fb_used = true;
        }
        ok
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

        // Check if we could render pixel-perfect QR to framebuffer
        self.render_qr_fb_once(8);

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
        if self.fb_used {
            // FB QR was drawn — use empty string so ratatui draws nothing in this area
            lines.push(Line::raw(""));
        } else if !qr_lines.is_empty() {
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

    /// Convert rendered QR Lines back into a binary grid and decode with rxing.
    /// Handles both Linux (`#`/space) and macOS (half-block chars) output.
    fn decode_qr_lines(lines: &[Line<'_>], force_2x_width: bool) -> Result<String, String> {
        use rxing::Reader;

        if lines.is_empty() {
            return Err("no lines".into());
        }

        let term_height = lines.len();
        let term_width: usize = lines.iter().map(|l| {
            l.spans.iter().map(|s| s.content.chars().count()).sum::<usize>()
        }).max().unwrap_or(0);

        if term_width == 0 {
            return Err("zero width".into());
        }

        let upscale = 4u32;
        let pixel_width: u32;
        let pixel_height: u32;

        // Check if we have half-block chars
        let has_half_block = lines.iter().any(|l| {
            l.spans.iter().any(|s| {
                s.content.chars().any(|c| matches!(c, '\u{2588}' | '\u{2580}' | '\u{2584}'))
            })
        });

        if has_half_block || force_2x_width {
            // Half-block: 1 pixel wide, 2 pixels tall per char
            pixel_width = (term_width as u32) * upscale;
            pixel_height = (term_height as u32) * 2 * upscale;
        } else {
            // `#`/space: 1 pixel wide, 1 pixel tall per char
            pixel_width = (term_width as u32) * upscale;
            pixel_height = (term_height as u32) * upscale;
        }

        let mut pixels = vec![255u8; (pixel_width * pixel_height) as usize];

        if has_half_block {
            for (row_idx, line) in lines.iter().enumerate() {
                let top_y = (row_idx as u32) * 2 * upscale;
                let bot_y = top_y + upscale;
                let mut x = 0u32;
                for span in &line.spans {
                    for ch in span.content.chars() {
                        let (top_dark, bot_dark) = match ch {
                            '\u{2588}' => (true, true),
                            '\u{2580}' => (true, false),
                            '\u{2584}' => (false, true),
                            _ => (false, false),
                        };
                        for dy in 0..upscale {
                            for dx in 0..upscale {
                                let px = x * upscale + dx;
                                let py_top = top_y + dy;
                                let py_bot = bot_y + dy;
                                if top_dark && py_top < pixel_height {
                                    pixels[(py_top * pixel_width + px) as usize] = 0;
                                }
                                if bot_dark && py_bot < pixel_height {
                                    pixels[(py_bot * pixel_width + px) as usize] = 0;
                                }
                            }
                        }
                        x += 1;
                    }
                }
            }
        } else {
            for (row_idx, line) in lines.iter().enumerate() {
                let y = (row_idx as u32) * upscale;
                let mut x = 0u32;
                for span in &line.spans {
                    for ch in span.content.chars() {
                        let is_dark = ch == '#';
                        for dy in 0..upscale {
                            for dx in 0..upscale {
                                let px = x * upscale + dx;
                                let py = y + dy;
                                pixels[(py * pixel_width + px) as usize] = if is_dark { 0 } else { 255 };
                            }
                        }
                        x += 1;
                    }
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
        // Test macOS half-block path (TERM != "linux")
        let url = "https://example.com/test";
        let lines = SystemIdModule::render_qr_lines(url, 120, 40);
        assert!(!lines.is_empty(), "render_qr_lines returned empty");
        let decoded = decode_qr_lines(&lines, false)
            .expect("QR code at scale=1 should be decodable");
        assert_eq!(decoded, url);
    }

    #[test]
    fn qr_render_widget_size_short_url_decodable() {
        let url = "https://example.com/r?id=a";
        let lines = SystemIdModule::render_qr_lines(url, 118, 25);
        assert!(!lines.is_empty(), "short URL QR should fit in widget");
        let decoded = decode_qr_lines(&lines, false)
            .expect("Short URL QR should be decodable at widget size");
        assert_eq!(decoded, url);
    }

    #[test]
    fn qr_render_widget_size_long_url_graceful() {
        // Long URL that can't fit in widget — should return empty (no broken QR)
        let url = "https://debug.consolving.net/system?register=eyJzeXN0ZW1faWQiOiJiNGVhZjI1YS1lMWQ5LTQwNDgtYWRjMS0xYjc0MjdiZmQ2NjIiLCJmaW5nZXJwcmludCI6InVua25vd258MDI6NDI6YWM6MTI6MDA6MDB8MjUuNC4wIn0";
        let lines = SystemIdModule::render_qr_lines(url, 118, 18);
        if !lines.is_empty() {
            let decoded = decode_qr_lines(&lines, false)
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
        let decoded = decode_qr_lines(&lines, false)
            .expect("QR code with realistic payload should be decodable at full page size");
        assert_eq!(decoded, url);
    }

    #[test]
    fn qr_render_linux_hash_path_decodable() {
        // Test Linux `#` rendering path
        let url = "https://example.com/test?long";
        let lines = SystemIdModule::render_qr_lines(url, 120, 40);
        assert!(!lines.is_empty(), "render_qr_lines returned empty");
        let decoded = decode_qr_lines(&lines, true)
            .expect("Linux QR with # should be decodable");
        assert_eq!(decoded, url);
    }
}
