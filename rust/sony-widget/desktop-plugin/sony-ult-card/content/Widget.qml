import QtQuick
import Ryoku.PluginKit
import Ryoku.PluginKit.Singletons

// Desktop card in the style of the music card: a cover tile, title, dotted
// indicator, wavy battery line and round noise-control buttons.
Item {
    id: w
    property var pluginApi: null
    property var screen: null
    property string density: "compact"
    property real s: 1
    property real widthBudget: 360
    property bool active: true

    readonly property var svc: pluginApi ? pluginApi.mainInstance : null
    readonly property bool up: !!(svc && svc.connected)
    readonly property int battery: up && svc.battery >= 0 ? svc.battery : 0

    implicitWidth: 420 * s
    implicitHeight: 208 * s

    // Same tiers as the bar icon.
    readonly property color tint: !up ? Theme.dim
        : svc.battery < 0 ? Theme.dim
        : svc.battery >= 60 ? "#ffffff"
        : svc.battery > 30 ? "#f9e2af"
        : svc.battery >= 10 ? "#e5484d" : "#ff1f1f"

    readonly property string modeLabel: !up ? "Not connected"
        : svc.mode === "nc" ? "Noise cancelling"
        : svc.mode === "ambient" ? "Ambient sound" : "Noise control off"
    readonly property string subtitle: !up ? "Turn the headphones on"
        : [modeLabel, svc.codec].filter(x => x).join("  ·  ")
    readonly property string ultText: !up || !svc.caps || svc.caps.ult !== true || svc.ultMode < 0 ? ""
        : (svc.ultMode === 0 ? "ULT off" : "ULT " + svc.ultMode)
          + (svc.eqPresetName ? "  ·  " + svc.eqPresetName : "")

    // ── cover tile ───────────────────────────────────────────────────────────
    Rectangle {
        id: cover
        x: 0; y: 0
        width: 112 * w.s; height: 112 * w.s
        radius: 18 * w.s
        border.width: 1
        border.color: Theme.border
        gradient: Gradient {
            GradientStop { position: 0.0; color: Qt.rgba(Theme.accent.r, Theme.accent.g, Theme.accent.b, w.up ? 0.55 : 0.18) }
            GradientStop { position: 1.0; color: Qt.rgba(Theme.accent.r, Theme.accent.g, Theme.accent.b, w.up ? 0.12 : 0.05) }
        }

        Canvas {
            id: art
            anchors.centerIn: parent
            width: 70 * w.s
            height: 64 * w.s
            property color tint: w.tint
            onTintChanged: requestPaint()
            onWidthChanged: requestPaint()
            onPaint: {
                const ctx = getContext("2d");
                ctx.reset();
                ctx.scale(width / 24, width / 24);   // drawn on a 24 x 22 grid
                ctx.strokeStyle = tint;
                ctx.fillStyle = tint;
                ctx.lineWidth = 2.4;
                ctx.lineCap = "round";
                ctx.beginPath();
                ctx.arc(12, 12, 8.2, Math.PI, 2 * Math.PI, false);
                ctx.stroke();
                function cup(x) {
                    const y = 11, cw = 6.4, ch = 10.5, r = 2.6;
                    ctx.beginPath();
                    ctx.moveTo(x + r, y);
                    ctx.lineTo(x + cw - r, y);
                    ctx.quadraticCurveTo(x + cw, y, x + cw, y + r);
                    ctx.lineTo(x + cw, y + ch - r);
                    ctx.quadraticCurveTo(x + cw, y + ch, x + cw - r, y + ch);
                    ctx.lineTo(x + r, y + ch);
                    ctx.quadraticCurveTo(x, y + ch, x, y + ch - r);
                    ctx.lineTo(x, y + r);
                    ctx.quadraticCurveTo(x, y, x + r, y);
                    ctx.closePath();
                    ctx.fill();
                }
                cup(0.9);
                cup(24 - 0.9 - 6.4);
            }
        }
    }

    // ── top-right: ULT / preset label, then the dotted battery line ──────────
    Text {
        id: ultLabel
        anchors.right: parent.right
        y: 2 * w.s
        visible: text !== ""
        text: w.ultText
        color: Theme.dim
        font.family: Theme.font
        font.pixelSize: 11 * w.s
    }

    Row {
        id: dots
        x: cover.width + 18 * w.s
        width: parent.width - x
        y: 50 * w.s
        spacing: 0
        readonly property int count: 34
        readonly property real dot: 4.2 * w.s
        Repeater {
            model: dots.count
            delegate: Item {
                required property int index
                width: dots.width / dots.count
                height: dots.dot
                Rectangle {
                    anchors.centerIn: parent
                    width: dots.dot; height: dots.dot; radius: width / 2
                    color: index < Math.round(dots.count * w.battery / 100)
                        ? Theme.accent : Qt.rgba(Theme.accent.r, Theme.accent.g, Theme.accent.b, 0.25)
                }
            }
        }
    }

    // ── title block under the cover ──────────────────────────────────────────
    Column {
        x: 0
        y: cover.height + 12 * w.s
        spacing: 3 * w.s
        Text {
            text: w.up && w.svc.name ? w.svc.name : "Sony headphones"
            color: Theme.bright
            font.family: Theme.font
            font.pixelSize: 20 * w.s
        }
        Text {
            text: w.subtitle
            color: Theme.dim
            font.family: Theme.font
            font.pixelSize: 12 * w.s
        }
    }

    // ── bottom row: wavy battery line + percentage, noise-control buttons ────
    Text {
        id: pct
        anchors.right: buttons.left
        anchors.rightMargin: 14 * w.s
        y: parent.height - 52 * w.s
        text: w.up && w.svc.battery >= 0 ? w.svc.battery + "%" + (w.svc.charging ? "  ⚡" : "") : "—"
        color: Theme.dim
        font.family: Theme.font
        font.pixelSize: 12 * w.s
    }
    WaveMeter {
        id: wave
        x: 0
        y: parent.height - 24 * w.s
        width: buttons.x - 14 * w.s
        s: w.s
        frac: w.battery / 100
    }

    Row {
        id: buttons
        anchors.right: parent.right
        y: parent.height - 42 * w.s
        spacing: 8 * w.s

        component ModeButton: Rectangle {
            id: mb
            property string mode: ""
            property string glyph: ""
            readonly property bool on: w.up && w.svc.mode === mode
            width: 34 * w.s; height: 34 * w.s; radius: width / 2
            opacity: w.up ? 1 : 0.4
            color: on ? Theme.accent : (ma.containsMouse ? Qt.rgba(1, 1, 1, 0.12) : Qt.rgba(1, 1, 1, 0.06))
            border.width: on ? 0 : 1
            border.color: Theme.border
            Text {
                anchors.centerIn: parent
                text: mb.glyph
                color: mb.on ? "#10131a" : Theme.bright
                font.family: Theme.font
                font.bold: true
                font.pixelSize: 11 * w.s
            }
            MouseArea {
                id: ma
                anchors.fill: parent
                hoverEnabled: true
                cursorShape: Qt.PointingHandCursor
                onClicked: if (w.up) w.svc.setNoise(mb.mode)
            }
        }
        ModeButton { mode: "nc"; glyph: "NC" }
        ModeButton { mode: "ambient"; glyph: "AMB" }
        ModeButton { mode: "off"; glyph: "OFF" }
    }
}
