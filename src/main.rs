use sys_wall::config::Config;
use sys_wall::modules;
use sys_wall::{Module, ModuleCapability};
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use crossterm::execute;
use ratatui::backend::CrosstermBackend;
use ratatui::prelude::Stylize;
use ratatui::Terminal;
use std::io::{self, Write};

const TICK_RATE_MS: u64 = 1000;

/// Get the list of (index, name) tuples for page-capable modules.
fn page_module_list(modules: &[std::boxed::Box<dyn Module>]) -> Vec<(usize, String)> {
    modules
        .iter()
        .enumerate()
        .filter(|(_, m)| {
            matches!(
                m.capability(),
                ModuleCapability::WidgetAndPage | ModuleCapability::PageOnly
            )
        })
        .map(|(i, m)| (i, m.name().to_string()))
        .collect()
}

fn clear_framebuffer() {
    let _ = io::stdout().write_all(b"\x1b[2J\x1b[H");
    let _ = io::stdout().flush();
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    clear_framebuffer();
    let config = Config::load()?;
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut modules = modules::register_modules();

    // Build a static list of page names (used only for keyboard switching)
    let page_names: Vec<String> = {
        let list = page_module_list(&modules);
        list.into_iter().map(|(_, nm)| nm).collect()
    };
    let page_count = page_names.len();

    let mut current_page: usize = 0;

    loop {
        let ctx = sys_wall::SystemContext::new(config.clone());

        for module in modules.iter_mut() {
            let _ = module.update(&ctx);
        }

        terminal.draw(|frame| {
            let area = frame.area();
            let tab_y = area.height.saturating_sub(1);
            let main_area = ratatui::layout::Rect::new(0, 0, area.width, tab_y);

            // Render all widgets as a grid on the Dashboard page
            let widget_modules: Vec<_> = modules
                .iter()
                .filter(|m| {
                    matches!(
                        m.capability(),
                        ModuleCapability::WidgetAndPage | ModuleCapability::PageOnly | ModuleCapability::WidgetOnly
                    ) && m.name() != "Dashboard"
                })
                .map(|m| m.as_ref())
                .collect();

            if !widget_modules.is_empty() {
                let mut widgets_per_row: u16 = 1;
                if main_area.width >= 46 * 2 {
                    widgets_per_row = (main_area.width / 46).min(3);
                    if widgets_per_row < 2 {
                        widgets_per_row = 2;
                    }
                }

                let mut row_heights: Vec<u16> = Vec::new();
                let mut idx: u16 = 0;
                while idx < widget_modules.len() as u16 {
                    let mut max_h: u16 = 0;
                    for _ in 0..widgets_per_row {
                        if idx < widget_modules.len() as u16 {
                            let h = widget_modules[idx as usize].widget_height();
                            if h > max_h {
                                max_h = h.max(4);
                            }
                            idx += 1;
                        }
                    }
                    row_heights.push(max_h.max(4));
                }

                let layout = ratatui::layout::Layout::default()
                    .direction(ratatui::layout::Direction::Vertical)
                    .constraints(
                        row_heights
                            .iter()
                            .map(|h| ratatui::layout::Constraint::Length(*h))
                            .collect::<Vec<_>>(),
                    );

                let regions = layout.split(main_area);

                let mut cell: u16 = 0;
                for region in regions.iter() {
                    for col in 0..widgets_per_row {
                        if cell < widget_modules.len() as u16 {
                            let cell_w = region.width / widgets_per_row;
                            let cell_area = ratatui::layout::Rect::new(
                                region.left() + col * cell_w,
                                region.top(),
                                cell_w.max(1),
                                region.height.max(1),
                            );
                            widget_modules[cell as usize].render_widget(frame, cell_area);
                            cell += 1;
                        }
                    }
                }
            }

            // Render simplified tab bar
            render_tab_bar(frame, &page_names, current_page);
        })?;

        if event::poll(std::time::Duration::from_millis(TICK_RATE_MS))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match key.code {
                        KeyCode::Char('q') | KeyCode::Esc => {
                            disable_raw_mode()?;
                            execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
                            terminal.show_cursor()?;
                            return Ok(());
                        }
                        KeyCode::Char('1') => {
                            if page_count > 0 {
                                current_page = 0;
                            }
                        }
                        KeyCode::Char('2') => {
                            if page_count > 1 {
                                current_page = 1;
                            }
                        }
                        KeyCode::Char('3') => {
                            if page_count > 2 {
                                current_page = 2;
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
    }
}

fn render_tab_bar<'a>(
    frame: &mut ratatui::Frame<'a>,
    page_names: &[String],
    current_page: usize,
) {
    let area = frame.area();
    let tab_y = area.height.saturating_sub(1);

    if page_names.is_empty() {
        return;
    }

    let mut spans: Vec<_> = Vec::new();
    spans.push(ratatui::text::Span::styled(
        " sys-wall ",
        ratatui::style::Style::default()
            .fg(ratatui::style::Color::Yellow)
            .bold(),
    ));
    spans.push(ratatui::text::Span::raw(" | "));

    for (i, name) in page_names.iter().enumerate() {
        let is_active = i == current_page;
        let key = (i as u8) + 1;
        let text = format!("[{key}:{}] ", name.as_str());
        let tab_span = if is_active {
            ratatui::text::Span::styled(
                text,
                ratatui::style::Style::default()
                    .fg(ratatui::style::Color::Green)
                    .bold(),
            )
        } else {
            ratatui::text::Span::raw(text)
        };
        spans.push(tab_span);
    }

    let tab_line = ratatui::text::Line::from(spans);
    let tab_area = ratatui::layout::Rect::new(0, tab_y, area.width, 1);
    frame.render_widget(ratatui::widgets::Paragraph::new(tab_line), tab_area);
}
