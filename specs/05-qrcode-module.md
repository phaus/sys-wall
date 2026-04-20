# QR Code Module (F4)

## Purpose

Display a QR code on the console that encodes system identification data. Scanning the QR code triggers a registration/provisioning flow by POSTing data to a preconfigured URL.

## Layout

```
┌──────────────────────────────────────────────┐
│              System Registration             │
├──────────────────┬───────────────────────────┤
│                  │  UUID:     xxxx-xxxx-...  │
│   ██ ██ ██ ██    │  Hostname: node-01        │
│   ██    ██  ██   │  IP:       10.0.2.15      │
│   ██ ██ ██ ██    │  MAC:      aa:bb:cc:...   │
│   ██    ██  ██   │  Arch:     x86_64         │
│   ██ ██ ██ ██    │  OS:       Linux 6.x      │
│                  │                           │
│  (QR Code)       │  Target URL:             │
│                  │  https://example.com/reg  │
├──────────────────┴───────────────────────────┤
│ [R] Refresh  [P] POST Now  [C] Copy Payload │
└──────────────────────────────────────────────┘
```

## QR Code Content

The QR code encodes a URL with query parameters or a JSON payload:

```
https://example.com/register?uuid=XXXX&hostname=node-01&ip=10.0.2.15&mac=aa:bb:cc:dd:ee:ff
```

Or the QR code directly encodes a JSON blob:

```json
{
  "uuid": "xxxx-xxxx-xxxx",
  "hostname": "node-01",
  "ip": "10.0.2.15",
  "mac": "aa:bb:cc:dd:ee:ff",
  "arch": "x86_64",
  "kernel": "6.1.0"
}
```

## Rendering

- Use the `qrcode` crate to generate a QR matrix
- Render using Unicode block characters (`█`, `▀`, `▄`, ` `) for 2-row-per-character density
- Ensure QR code fits within available terminal space

## POST Flow

When the user presses `P`:
1. Collect system info into JSON payload
2. HTTP POST to the configured URL using `ureq`
3. Display response status (success/error) in the UI
4. Optionally retry on failure

## Configuration

```toml
[qrcode]
target_url = "https://example.com/api/register"
mode = "url"          # "url" (QR = URL with params) | "json" (QR = raw JSON)
auto_post = false     # Automatically POST on boot
extra_fields = {}     # Additional static key-value pairs to include
```
