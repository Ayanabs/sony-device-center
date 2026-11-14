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
        Text {
            anchors.verticalCenter: parent.verticalCenter
            font.family: "JetBrainsMono Nerd Font Mono"
            font.pixelSize: 20
            color: w.tint
            text: ""          // headphones
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
