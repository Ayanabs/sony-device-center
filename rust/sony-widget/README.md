# sony-widget — Waybar module

Tiny (~3 MB RSS) Waybar module for Sony headphones. It holds the headset's RFCOMM
connection, listens for the headset's push notifications and prints a Waybar JSON
line only when something changes (battery, noise-control mode, connect/disconnect).

> The headset allows one RFCOMM client at a time. Close the Qt app / `sonyd`
> before starting the widget, and vice versa.

## Install

```sh
cd rust && cargo build --release -p sony-widget
install -Dm755 target/release/sony-widget ~/.local/bin/sony-widget
```

Needs `bluez` running and `bluez-libs` (libbluetooth). The headset must be paired
and connected (`bluetoothctl connect <MAC>`); the widget auto-detects the first
connected device that exposes a Sony service, or pass `--addr AA:BB:CC:DD:EE:FF`.

## Waybar

`~/.config/waybar/config.jsonc`:

```jsonc
"modules-right": ["custom/sony", "..."],
"custom/sony": {
  "exec": "sony-widget",
  "return-type": "json",
  "format": "{}",
  "on-click": "sony-widget cycle-nc",
  "on-click-right": "sony-widget set-nc off",
  "tooltip": true
}
```

`~/.config/waybar/style.css`:

```css
#custom-sony.disconnected { opacity: 0.4; }
#custom-sony.charging     { color: #a6e3a1; }
#custom-sony.low          { color: #f38ba8; }
#custom-sony.ambient      { color: #f9e2af; }
```

The glyphs are Nerd Font icons (󰋋 󰋎 󰟎); use a Nerd Font in the bar.

## Commands (talk to the running instance via `$XDG_RUNTIME_DIR/sony-widget.sock`)

| Command | Effect |
| --- | --- |
| `sony-widget cycle-nc` | noise cancelling → ambient → off → … |
| `sony-widget set-nc off\|nc\|ambient` | set a mode directly |
| `sony-widget status` | print the current JSON line |
