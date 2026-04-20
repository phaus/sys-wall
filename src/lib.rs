pub mod config;
pub mod modules;

pub use config::Config;
pub use crossterm::event::{Event, KeyCode};

/// Shared data passed to all modules on each update cycle.
#[derive(Clone)]
pub struct SystemContext {
    pub hostname: String,
    pub uuid: String,
    pub uptime: Duration,
    pub cpu_usage: f32,
    pub cpu_per_core: Vec<f32>,
    pub memory_used: u64,
    pub memory_total: u64,
    pub load_avg: (f64, f64, f64),
    pub kernel_version: String,
    pub config: std::sync::Arc<Config>,
}

use std::time::Duration;

impl SystemContext {
    pub fn new(config: Config) -> Self {
        let hostname = std::fs::read_to_string("/etc/hostname")
            .map(|h| h.trim().to_string())
            .unwrap_or_else(|_| "unknown".to_string());

        let uuid = uuid::Uuid::new_v4().to_string();

        let kernel = std::env::var("KERNEL_VERSION").unwrap_or_else(|_| "unknown".to_string());

        Self {
            hostname,
            uuid,
            uptime: Duration::ZERO,
            cpu_usage: 0.0,
            cpu_per_core: Vec::new(),
            memory_used: 0,
            memory_total: 0,
            load_avg: (0.0, 0.0, 0.0),
            kernel_version: kernel,
            config: std::sync::Arc::new(config),
        }
    }
}

/// Defines what a module can render.
#[derive(PartialEq)]
pub enum ModuleCapability {
    PageOnly,
    WidgetOnly,
    WidgetAndPage,
}

/// Size hint for widget layout on the Dashboard.
pub enum WidgetSize {
    Small,
    Medium,
    Large,
}

/// Trait that every sys-wall module must implement.
pub trait Module {
    fn name(&self) -> &str;

    fn keybinding(&self) -> Option<KeyCode>;

    fn capability(&self) -> ModuleCapability;

    fn widget_size(&self) -> WidgetSize {
        WidgetSize::Small
    }

    fn update(&mut self, ctx: &SystemContext) -> Result<(), Box<dyn std::error::Error>>;

    fn render_widget(&self, frame: &mut ratatui::Frame<'_>, area: ratatui::layout::Rect);

    fn render_page(&self, frame: &mut ratatui::Frame<'_>, area: ratatui::layout::Rect);

    fn handle_input(&mut self, _event: &Event) -> Result<bool, Box<dyn std::error::Error>> {
        Ok(false)
    }
}
