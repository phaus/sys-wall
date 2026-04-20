# Monitor Module (F2)

## Purpose

Real-time system resource monitoring with graphs and a process list.

## Layout

```
┌─ Header Bar ───────────────────────────────────────────────┐
├────────────┬───────────────┬──────────────┬────────────────┤
│ CPU        │ CPU Detail    │ LOAD         │ MEMORY         │
│  0.1%      │ User   0.1%  │ 1m    0.01   │ Total   3 GB   │
│ MEM        │ System 0.0%  │ 5m    0.01   │ Used  141 MB   │
│  4.7%      │ Idle  99.9%  │ 15m   0.00   │ Free  2.6 GB   │
├────────────┴──────┬────────┴──────────────┴────────────────┤
│ CPU USER/SYSTEM   │ MEM USED        │ LOAD AVG 60s        │
│ [sparkline graph] │ [bar graph]     │ [sparkline graph]    │
├───────────────────┴─────────────────┴──────────────────────┤
│ NET RX/TX         │ PID  S  CPU%  MEM%  COMMAND            │
│ [bar graphs]      │ 1441 S  0.2   3.0   /sbin/dashboard    │
│                   │ 1    S  0.0   3.3   /sbin/init         │
├───────────────────┤                                        │
│ DISK READ/WRITE   │                                        │
│ [bar graphs]      │                                        │
└───────────────────┴────────────────────────────────────────┘
```

## Data Sources

- **CPU**: `sysinfo` crate — per-core and aggregate usage
- **Memory**: `sysinfo` — total, used, free, buffers, cache, shared
- **Load**: `/proc/loadavg`
- **Processes**: `sysinfo` — top N by CPU usage
- **Network**: `/proc/net/dev` or `sysinfo` — RX/TX bytes delta
- **Disk**: `/proc/diskstats` or `sysinfo` — read/write bytes delta

## Graphs

- Use ratatui `Sparkline` widget for time-series (CPU, load)
- Use ratatui `BarChart` or custom bars for network/disk I/O
- Retain last 60 data points (1 per second = 60s window)

## Update Interval

- Graphs: 1 second
- Process list: 2 seconds
- Stats header: 1 second

## Configuration

```toml
[monitor]
update_interval_ms = 1000
process_count = 10          # Number of top processes to show
history_seconds = 60        # Graph history window
```
