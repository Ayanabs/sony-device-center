import QtQuick

// Bar glyph: headphones icon + battery %. Icon colour = battery level.
//   left click  -> open the panel (noise control, EQ, features, devices)
//   right click -> cycle noise cancelling / ambient / off
Item {
    id: w
    property var pluginApi: null
    property string density: "glyph"
    property real widthBudget: 220
    property bool active: true

    readonly property var svc: pluginApi ? pluginApi.mainInstance : null
    readonly property bool up: svc && svc.connected

    // Hidden while the headset is off so the bar stays tidy.
    implicitWidth: up ? row.implicitWidth : 0
    implicitHeight: 32
    visible: implicitWidth > 0

    // Icon colour follows the battery: white at 60% and above, yellow from 59% down
    // to 31%, red from 30% to 10%, bright red below 10%. Grey when the level is unknown.
    readonly property color tint: !up || svc.battery < 0 ? "#808080"
        : svc.battery >= 60 ? "#ffffff"
        : svc.battery > 30 ? "#f9e2af"
        : svc.battery >= 10 ? "#e5484d" : "#ff1f1f"
    // The percentage stays neutral, and turns green while charging.
    readonly property color battTint: !up ? "#808080" : svc.charging ? "#a6e3a1" : "#d0d0d0"

    Row {
        id: row
        anchors.verticalCenter: parent.verticalCenter
        spacing: 5
        // Hand-drawn headphones: a slim band and thick, filled ear cups.
        Canvas {
            id: icon
            anchors.verticalCenter: parent.verticalCenter
            width: 17
            height: 15.6
            property color tint: w.tint
            onTintChanged: requestPaint()
            onPaint: {
                const ctx = getContext("2d");
                ctx.reset();
                ctx.scale(width / 24, width / 24);   // drawn on a 24 x 22 grid
                ctx.strokeStyle = tint;
                ctx.fillStyle = tint;
                ctx.lineWidth = 2.4;
                ctx.lineCap = "round";
                // band
                ctx.beginPath();
                ctx.arc(12, 12, 8.2, Math.PI, 2 * Math.PI, false);
                ctx.stroke();
                // cups: 6 x 11 rounded blocks under the band ends
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
        Text {
            anchors.verticalCenter: parent.verticalCenter
            visible: w.up && w.svc.battery >= 0
            font.pixelSize: 12
            font.family: "JetBrainsMono Nerd Font Mono"
            color: w.battTint
            text: w.up ? w.svc.battery + "%" : ""
        }
    }

    MouseArea {
        anchors.fill: parent
        acceptedButtons: Qt.LeftButton | Qt.RightButton
        cursorShape: Qt.PointingHandCursor
        onClicked: mouse => {
            if (!w.svc) return;
            if (mouse.button === Qt.RightButton) w.svc.cycleNoiseControl();
            else if (w.pluginApi) w.pluginApi.togglePanel();
        }
    }
}
