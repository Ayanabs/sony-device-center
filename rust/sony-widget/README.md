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
| `sony-widget set-ambient LEVEL [on\|off]` | ambient level 1-20, optional voice focus |
| `sony-widget set-eq PRESET` | EQ preset id (`0x16`) or name (`bass-boost`) |
| `sony-widget set-pause on\|off` | pause on removal (if supported) |
| `sony-widget set-dsee on\|off` | DSEE (if supported) |

## Ryoku `qsbar` plugin (headphone icon in the bar)

`bar-plugin/sony-ult` is a `topbarGlyph` plugin. It runs `sony-widget` itself, so do
not also run the Waybar module or the corner pill (the headset allows one connection).

```sh
mkdir -p ~/.local/share/ryoku-dev-plugins && cp -r bar-plugin/sony-ult ~/.local/share/ryoku-dev-plugins/
# the shell must see the folder: put this in ~/.config/hypr/user.lua, then log in again
#   hl.env("RYOSTORE_PLUGINS_DIR", os.getenv("HOME") .. "/.local/share/ryoku-dev-plugins")
RYOSTORE_PLUGINS_DIR=~/.local/share/ryoku-dev-plugins ryoku-plugins-place sony-ult host topbarGlyph
RYOSTORE_PLUGINS_DIR=~/.local/share/ryoku-dev-plugins ryoku-plugins-place sony-ult enabled true
```

Left-click opens a panel (noise control with ambient level and voice focus, equalizer
presets, pause on removal, DSEE, connected devices); right-click cycles noise control.
Move it next to the music widget with `ryoku-shell bar move sony-ult --section right --index 1`.
