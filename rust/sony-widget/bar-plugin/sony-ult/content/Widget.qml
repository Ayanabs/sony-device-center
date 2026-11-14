import QtQuick

// Bar glyph: headphones icon + battery %, tinted by noise-control mode.
//   left click  -> cycle noise cancelling / ambient / off
//   right click -> noise control off
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

    readonly property color tint: !up ? "#808080"
        : svc.mode === "nc" ? "#8ab4f8"
        : svc.mode === "ambient" ? "#f9e2af" : "#d0d0d0"
    readonly property color battTint: !up ? "#808080"
        : svc.charging ? "#a6e3a1" : (svc.battery >= 0 && svc.battery <= 15) ? "#f38ba8" : "#d0d0d0"

    Row {
        id: row
        anchors.verticalCenter: parent.verticalCenter
        spacing: 5
        Text {
            anchors.verticalCenter: parent.verticalCenter
            font.family: "JetBrainsMono Nerd Font Mono"
            font.pixelSize: 15
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
            if (mouse.button === Qt.RightButton) w.svc.noiseControlOff();
            else w.svc.cycleNoiseControl();
        }
    }
}
