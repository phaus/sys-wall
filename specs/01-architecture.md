# Architecture

## Overview

sys-wall is a single Rust binary that renders a tabbed TUI on the Linux framebuffer console. It is designed for headless servers and embedded systems where no desktop environment is available.

## High-Level Architecture

```
┌─────────────────────────────────────────────────────┐
│                     sys-wall                        │
├─────────────────────────────────────────────────────┤
│                  Dashboard (F1)                     │
│  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌────────┐ │
│  │ Widget A │ │ Widget B │ │ Widget C │ │ Wgt D  │ │  <- render_widget()
│  └──────────┘ └──────────┘ └──────────┘ └────────┘ │
├──────────┬──────────┬───────────┬───────────────────┤
│ Monitor  │ Network  │ QR Code   │ Logs              │
│ Page     │ Page     │ Page      │ Page              │  <- render_page()
├──────────┴──────────┴───────────┴───────────────────┤
│              Module Manager                         │
│  (registers modules, routes events, layout engine)  │
├─────────────────────────────────────────────────────┤
│              Core Framework                         │
│  ┌────────┐ ┌──────────┐ ┌────────────────┐         │
│  │ Config │ │ Event    │ │ SystemContext   │         │
│  │ Loader │ │ Handler  │ │ (shared data)  │         │
│  └────────┘ └──────────┘ └────────────────┘         │
├─────────────────────────────────────────────────────┤
│         Ratatui + Crossterm Backend                 │
└─────────────────────────────────────────────────────┘
```

## Module Duality: Widget + Page

Every module can provide:

- **Widget** — compact view rendered on the Dashboard (F1)
- **Page** — full-screen view on its own tab (activated by F-key)
- **Both** — a widget summary on Dashboard + a detailed page

See [08-module-system.md](08-module-system.md) for the `Module` trait and layout engine.

## Module Trait (Summary)

```rust
pub trait Module {
    fn name(&self) -> &str;
    fn keybinding(&self) -> Option<KeyCode>;
    fn capability(&self) -> ModuleCapability;  // PageOnly | WidgetOnly | WidgetAndPage
    fn widget_size(&self) -> WidgetSize;       // Small | Medium | Large
    fn update(&mut self, ctx: &SystemContext) -> Result<()>;
    fn render_widget(&self, frame: &mut Frame, area: Rect);
    fn render_page(&self, frame: &mut Frame, area: Rect);
    fn handle_input(&mut self, event: &Event) -> Result<bool>;
}
```

## Key Design Decisions

- **Static binary**: Target `x86_64-unknown-linux-musl` for zero runtime dependencies
- **No async runtime**: Use synchronous I/O with polling to keep the binary small
- **crossterm backend**: No ncurses/terminfo dependency — works on raw framebuffer
- **Widget + Page duality**: Modules provide both a compact dashboard widget and a full page
- **Dashboard as layout engine**: F1 is not a module — it arranges all module widgets

## Data Flow

1. Main loop polls for terminal events (key presses, resize) and a tick timer (~1s)
2. On tick: each module's `update()` is called with fresh `SystemContext`
3. On render:
   - If Dashboard (F1) is active: call `render_widget()` on all widget-capable modules
   - If a module page is active: call that module's `render_page()`
   - Always render the header bar and tab bar
4. On input: route to active module's `handle_input()`; unhandled events go to framework (tab switching, quit)

## Dependencies (Minimal Set)

| Crate | Purpose |
|-------|---------|
| ratatui | TUI rendering |
| crossterm | Terminal backend |
| sysinfo | CPU, memory, disk, process info |
| nix | Network interface config (ioctl) |
| toml | Config file parsing |
| qrcode | QR code generation |
| ureq | HTTP POST (blocking, no async) |
| uuid | System UUID reading |
