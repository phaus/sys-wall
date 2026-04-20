# Module System

## Purpose

Allow extending sys-wall with new tabs without modifying the core binary. Each module can provide two rendering modes:

1. **Page** — A full-screen tab view, activated via F-key
2. **Widget** — A compact element displayed on the Dashboard page (F1)

A module may provide both, only a widget, or only a page.

## Core Concept: Widget + Page Duality

```
┌─ Dashboard (F1) ────────────────────────────────────────────┐
│ ┌─ System Status ──────┐ ┌─ Network ──────────────────────┐ │
│ │ Hostname: node-01    │ │ eth0  10.0.2.15/24  UP         │ │
│ │ Uptime:   3d 12h     │ │ eth1  192.168.1.5/24  UP       │ │
│ │ Users:    2           │ │ lo    127.0.0.1/8              │ │
│ │ Version:  1.2.0       │ └────────────────────────────────┘ │
│ └──────────────────────┘ ┌─ CPU / RAM ────────────────────┐ │
│ ┌─ Logs (dmesg) ───────┐ │ CPU ████████░░░░░░░░  45%      │ │
│ │ [4.012] usb 1-1: new │ │ RAM █████████████░░░  78%      │ │
│ │ [4.015] eth0: link up │ │ Load: 1.2  0.8  0.5           │ │
│ │ [4.102] ext4 mounted  │ └────────────────────────────────┘ │
│ │ ...                   │ ┌─ QR Code ─────────────────────┐ │
│ └───────────────────────┘ │ ██ ██ ██   Scan to register   │ │
│                           └────────────────────────────────┘ │
├─────────────────────────────────────────────────────────────┤
│ [Dashboard] --- [F2: Monitor] --- [F3: Network] --- ...    │
└─────────────────────────────────────────────────────────────┘
```

Each widget is a self-contained rectangle on the Dashboard. Pressing its F-key opens the full page.

## Module Trait

```rust
/// Defines what a module can render.
pub enum ModuleCapability {
    PageOnly,       // Only has a full-page view (e.g. Network Config)
    WidgetOnly,     // Only has a dashboard widget (e.g. simple status display)
    WidgetAndPage,  // Has both (e.g. Monitor: widget shows summary, page shows detail)
}

/// Size hint for widget layout on the Dashboard.
pub enum WidgetSize {
    Small,   // ~1/4 width, 4-6 rows   (e.g. CPU/RAM bar)
    Medium,  // ~1/2 width, 6-10 rows   (e.g. network summary)
    Large,   // full width, 8-15 rows    (e.g. log stream)
}

pub trait Module {
    /// Display name shown in tabs and widget titles.
    fn name(&self) -> &str;

    /// F-key to activate the full page view (None if WidgetOnly).
    fn keybinding(&self) -> Option<KeyCode>;

    /// What this module provides.
    fn capability(&self) -> ModuleCapability;

    /// Widget size hint for dashboard layout.
    fn widget_size(&self) -> WidgetSize { WidgetSize::Small }

    /// Called every tick. Collect/refresh data.
    fn update(&mut self, ctx: &SystemContext) -> Result<()>;

    /// Render the compact widget for the Dashboard page.
    /// Only called if capability is WidgetOnly or WidgetAndPage.
    fn render_widget(&self, frame: &mut Frame, area: Rect);

    /// Render the full page view.
    /// Only called if capability is PageOnly or WidgetAndPage.
    fn render_page(&self, frame: &mut Frame, area: Rect);

    /// Handle input when this module's page is active.
    fn handle_input(&mut self, event: &Event) -> Result<bool> {
        Ok(false)
    }
}
```

## Dashboard Layout Engine

The Dashboard (F1) is no longer a hardcoded module — it is a **layout engine** that collects all registered widgets and arranges them.

### Layout Algorithm

1. Collect all modules where `capability` is `WidgetOnly` or `WidgetAndPage`
2. Sort by configured `widget_order` (from config) or registration order
3. Lay out widgets in a grid:
   - `Large` widgets take full width
   - `Medium` widgets take half width (2 per row)
   - `Small` widgets take quarter width (4 per row, or fill remaining space)
4. Remaining vertical space goes to the last `Large` widget (typically Logs)

```
┌─────────────────────────────────────────────┐
│ Header Bar (always visible)                 │
├───────────┬───────────┬───────────┬─────────┤
│  Small    │  Small    │  Small    │ Small   │  <- row of small widgets
├───────────┴───────────┼───────────┴─────────┤
│  Medium               │  Medium             │  <- row of medium widgets
├───────────────────────┴─────────────────────┤
│  Large (fills remaining height)             │  <- e.g. log viewer
└─────────────────────────────────────────────┘
```

### Configuration

```toml
[dashboard]
# Order of widgets on the dashboard (by module name)
widget_order = ["system_status", "cpu_ram", "network", "logs"]

# Override size for specific widgets
[dashboard.widget_sizes]
logs = "large"
network = "medium"
```

## Built-in Modules

| Module | Widget | Page | Default Size | F-Key |
|--------|--------|------|-------------|-------|
| System Status | Hostname, uptime, version, users, UUID | — | Small | — |
| CPU / RAM | Usage bars, load average | Full monitor (graphs, processes) | Small | F2 |
| Network | Interface list with IPs, status | Network configuration form | Medium | F3 |
| Logs | Last N lines from dmesg/journal | Full scrollable log viewer | Large | F5 |
| QR Code | Mini QR + "Scan to register" | Full QR + details + POST button | Small | F4 |
| Disk | Usage bars per mount | Detailed I/O stats (future) | Small | — |

## Adding a New Module — Example

```rust
// src/modules/disk_usage.rs

pub struct DiskUsageModule {
    mounts: Vec<MountInfo>,
}

impl Module for DiskUsageModule {
    fn name(&self) -> &str { "Disk" }
    fn keybinding(&self) -> Option<KeyCode> { None } // Widget only, no page
    fn capability(&self) -> ModuleCapability { ModuleCapability::WidgetOnly }
    fn widget_size(&self) -> WidgetSize { WidgetSize::Small }

    fn update(&mut self, ctx: &SystemContext) -> Result<()> {
        self.mounts = collect_mount_info();
        Ok(())
    }

    fn render_widget(&self, frame: &mut Frame, area: Rect) {
        // Render compact disk usage bars
        for mount in &self.mounts {
            // draw gauge: /     ████████░░  80%
        }
    }

    fn render_page(&self, _frame: &mut Frame, _area: Rect) {
        // Not called — WidgetOnly
    }
}
```

Register it:
```rust
// src/modules/mod.rs
pub fn register_modules() -> Vec<Box<dyn Module>> {
    vec![
        Box::new(SystemStatusModule::new()),
        Box::new(CpuRamModule::new()),
        Box::new(NetworkModule::new()),
        Box::new(LogsModule::new()),
        Box::new(QrCodeModule::new()),
        Box::new(DiskUsageModule::new()),  // <-- add here
    ]
}
```

## Phase 1: Compile-Time Modules (MVP)

Modules are Rust structs implementing `Module`, registered at compile time in `register_modules()`.

## Phase 2: Dynamic Modules (Future)

Load modules as shared libraries (`.so`) at runtime using `libloading`. Intentionally deferred.

## SystemContext

Shared data passed to all modules on each update cycle:

```rust
pub struct SystemContext {
    pub hostname: String,
    pub uuid: String,
    pub uptime: Duration,
    pub cpu_usage: f32,
    pub cpu_per_core: Vec<f32>,
    pub memory: MemoryInfo,
    pub network: Vec<NetworkInterface>,
    pub disks: Vec<DiskInfo>,
    pub load_avg: (f64, f64, f64),
    pub processes: Vec<ProcessInfo>,
    pub logged_in_users: Vec<String>,
    pub kernel_version: String,
    pub config: Arc<Config>,
}
```
