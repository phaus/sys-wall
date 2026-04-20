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
use std::io;

const TICK_RATE_MS: u64 = 1000;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = Config::load()?;
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut modules = modules::register_modules();
    let mut current_tab: usize = 0;

    loop {
        let ctx = sys_wall::SystemContext::new(config.clone());

        for module in modules.iter_mut() {
            let _ = module.update(&ctx);
        }

        terminal.draw(|frame| {
    let area = frame.area();

            // Render active page
            let page_idx = current_tab;
            for (i, module) in modules.iter().enumerate() {
                if i == page_idx {
                    module.render_page(frame, area);
                }
            }

            // Render tab bar at bottom
            render_tabs(frame, &modules, current_tab);
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
                        KeyCode::F(1) => {
                            current_tab = 0;
                        }
                        _ => {
                            if let KeyCode::F(key_num) = key.code {
                                let tab_num = key_num as usize;
                                if tab_num >= 2 && tab_num <= modules.len() + 1 {
                                    current_tab = tab_num - 2;
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

fn render_tabs<'a>(
    frame: &mut ratatui::Frame<'a>,
    modules: &[std::boxed::Box<dyn Module>],
    current_tab: usize,
) {
    let area = frame.area();
    let tab_y = area.height.saturating_sub(1);

    let mut spans: Vec<_> = Vec::new();
    spans.push(ratatui::text::Span::raw("[Dashboard]---"));

    for (i, module) in modules.iter().enumerate() {
        if module.capability() != ModuleCapability::WidgetOnly {
            let is_active = i == current_tab;
            let key = ((i as u8) + 2) as char;
            let text = format!("[F{}:{}] ", key, module.name());
            let tab_span = if is_active {
                ratatui::text::Span::styled(
                    text,
                    ratatui::style::Style::default()
                        .fg(ratatui::style::Color::Yellow)
                        .bold(),
                )
            } else {
                ratatui::text::Span::raw(text)
            };
            spans.push(tab_span);
            spans.push(ratatui::text::Span::raw("---"));
        }
    }

    let tab_line = ratatui::text::Line::from(spans);
    let tab_area = ratatui::layout::Rect::new(0, tab_y, area.width, 1);
    frame.render_widget(ratatui::widgets::Paragraph::new(tab_line), tab_area);
}
