# Dashboard Page (F1)

## Purpose

The Dashboard is the default landing page. Instead of hardcoded content, it acts as a **layout engine** that arranges widgets provided by all registered modules.

## Layout

```
┌─ Header Bar (full width) ──────────────────────────────────┐
│ (hostname) (version): uptime Xm, CPUs, RAM, CPU%, RAM%    │
├──────────────────────┬─────────────────────────────────────┤
│ ┌─ System Status ──┐ │ ┌─ Network ───────────────────────┐ │
│ │ UUID   xxxxx     │ │ │ eth0  10.0.2.15/24  UP          │ │
│ │ Uptime 3d 12h    │ │ │ eth1  192.168.1.5   UP          │ │
│ │ Users  2         │ │ │ lo    127.0.0.1     UP          │ │
│ │ Kernel 6.1.0     │ │ └─────────────────────────────────┘ │
│ └──────────────────┘ │ ┌─ CPU / RAM ─────────────────────┐ │
│ ┌─ Disk ───────────┐ │ │ CPU ████████░░░░░░  45%         │ │
│ │ /   ████████ 80% │ │ │ RAM █████████████░  78%         │ │
│ │ /var ████░░░ 40% │ │ │ Load: 1.2  0.8  0.5            │ │
│ └──────────────────┘ │ └─────────────────────────────────┘ │
├──────────────────────┴─────────────────────────────────────┤
│ ┌─ Logs (dmesg) ─────────────────────────────────────────┐ │
│ │ [4.012] usb 1-1: new high-speed USB device             │ │
│ │ [4.015] eth0: link becomes ready                       │ │
│ │ [4.102] EXT4-fs: mounted filesystem                    │ │
│ │ ...                                          (scroll) │ │
│ └────────────────────────────────────────────────────────┘ │
├────────────────────────────────────────────────────────────┤
│ [Dashboard] --- [F2: Monitor] --- [F3: Network] --- ...   │
└────────────────────────────────────────────────────────────┘
```

## How It Works

The Dashboard is **not a regular module** — it is part of the core framework. On render:

1. Render the header bar (always present, on all pages)
2. Query all modules for their widgets (`capability` = `WidgetOnly` or `WidgetAndPage`)
3. Sort by `dashboard.widget_order` from config
4. Run the layout algorithm (see [08-module-system.md](08-module-system.md))
5. Call each module's `render_widget(frame, area)` with the assigned area

## Header Bar

The header bar is always visible on every page (Dashboard and module pages). It shows a single-line system summary:

```
(hostname) (v1.0.0): uptime 3d 12h, 4x3.5GHz, 8 GiB RAM, CPU 12%, RAM 45%
```

Source: `SystemContext` — updated every 1 second.

## Widget Focus

- `Tab` / `Shift+Tab`: Cycle focus between widgets
- Focused widget gets a highlighted border
- Arrow keys within a focused scrollable widget (e.g. Logs) scroll its content
- Pressing a module's F-key from the Dashboard opens its full page

## Default Widgets

These modules ship with built-in widgets:

| Module | Widget Content | Size |
|--------|---------------|------|
| System Status | UUID, hostname, uptime, kernel, users, version | Small |
| CPU / RAM | Usage bars + load average | Small |
| Network | Interface table (name, IP, status) | Medium |
| Disk | Mount point usage bars | Small |
| Logs | Scrollable dmesg/journal tail | Large |
| QR Code | Mini QR + registration hint | Small |

## Configuration

```toml
[dashboard]
widget_order = ["system_status", "cpu_ram", "network", "disk", "qrcode", "logs"]

[dashboard.widget_sizes]
logs = "large"
network = "medium"

[header]
show_cpu = true
show_ram = true
show_uptime = true
```

## Behavior

- All widgets update on the global tick (1s)
- Logs widget auto-scrolls unless the user has scrolled up
- Widgets that link to a page show a subtle `[F2]` indicator in their title
