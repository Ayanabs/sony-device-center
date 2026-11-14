import QtQuick

// Popup panel: everything the headset exposes, one click away.
Item {
    id: p
    property var pluginApi: null
    property string density: "full"
    property real widthBudget: 316
    property bool active: false
    property real s: 1

    readonly property var svc: pluginApi ? pluginApi.mainInstance : null
    readonly property var caps: svc ? svc.caps : ({})

    readonly property color ink: "#e8e8ea"
    readonly property color dim: "#9aa0a6"
    readonly property color chip: "#2b2e34"
    readonly property color chipHover: "#363a42"
    readonly property color accent: "#8ab4f8"
    readonly property string mono: "JetBrainsMono Nerd Font Mono"

    implicitHeight: col.implicitHeight

    readonly property var presets: [
        { id: 0x00, label: "Off" }, { id: 0x10, label: "Bright" }, { id: 0x11, label: "Excited" },
        { id: 0x12, label: "Mellow" }, { id: 0x13, label: "Relaxed" }, { id: 0x14, label: "Vocal" },
        { id: 0x15, label: "Treble Boost" }, { id: 0x16, label: "Bass Boost" }, { id: 0x17, label: "Speech" }
    ]

    // ---- reusable bits -----------------------------------------------------
    component Section: Text {
        color: p.dim
        font.pixelSize: 11
        font.letterSpacing: 1.2
        font.capitalization: Font.AllUppercase
        font.family: p.mono
    }

    component Chip: Rectangle {
        id: chipRoot
        property string label: ""
        property bool on: false
        property bool enabledChip: true
        signal clicked()
        implicitWidth: lbl.implicitWidth + 22
        implicitHeight: 30
        radius: 15
        opacity: enabledChip ? 1 : 0.4
        color: on ? Qt.rgba(0.54, 0.71, 0.97, 0.22) : (ma.containsMouse ? p.chipHover : p.chip)
        border.width: on ? 1 : 0
        border.color: p.accent
        Text {
            id: lbl
            anchors.centerIn: parent
            text: chipRoot.label
            color: chipRoot.on ? p.accent : p.ink
            font.pixelSize: 12
            font.family: p.mono
        }
        MouseArea {
            id: ma
            anchors.fill: parent
            hoverEnabled: true
            cursorShape: Qt.PointingHandCursor
            onClicked: if (chipRoot.enabledChip) chipRoot.clicked()
        }
    }

    component ToggleRow: Item {
        id: tr
        property string label: ""
        property string hint: ""
        property bool on: false
        signal toggled(bool value)
        implicitHeight: 40
        implicitWidth: 300
        Column {
            anchors.verticalCenter: parent.verticalCenter
            spacing: 1
            Text { text: tr.label; color: p.ink; font.pixelSize: 13; font.family: p.mono }
            Text { visible: tr.hint !== ""; text: tr.hint; color: p.dim; font.pixelSize: 10; font.family: p.mono }
        }
        Rectangle {
            id: track
            anchors.right: parent.right
            anchors.verticalCenter: parent.verticalCenter
            width: 40; height: 22; radius: 11
            color: tr.on ? p.accent : p.chip
            Rectangle {
                width: 16; height: 16; radius: 8; y: 3
                x: tr.on ? parent.width - width - 3 : 3
                color: tr.on ? "#10131a" : "#c8c8cc"
                Behavior on x { NumberAnimation { duration: 110 } }
            }
        }
        MouseArea {
            anchors.fill: parent
            cursorShape: Qt.PointingHandCursor
            onClicked: tr.toggled(!tr.on)
        }
    }

    Column {
        id: col
        width: parent.width
        spacing: 14

        // ---- header ---------------------------------------------------------
        Item {
            width: parent.width
            height: 46
            Text {
                id: gl
                anchors.verticalCenter: parent.verticalCenter
                text: ""; font.family: p.mono; font.pixelSize: 26
                color: p.svc && p.svc.mode === "nc" ? p.accent : p.ink
            }
            Column {
                anchors.left: gl.right; anchors.leftMargin: 12
                anchors.verticalCenter: parent.verticalCenter
                spacing: 2
                Text { text: p.svc ? p.svc.name : ""; color: p.ink; font.pixelSize: 15; font.bold: true; font.family: p.mono }
                Text {
                    color: p.dim; font.pixelSize: 11; font.family: p.mono
                    text: p.svc ? [p.svc.codec, p.svc.firmware ? "fw " + p.svc.firmware : ""].filter(x => x).join("  ·  ") : ""
                }
            }
            Column {
                anchors.right: parent.right
                anchors.verticalCenter: parent.verticalCenter
                spacing: 4
                Text {
                    anchors.right: parent.right
                    text: p.svc && p.svc.battery >= 0 ? p.svc.battery + "%" + (p.svc.charging ? "  ⚡" : "") : "—"
                    color: p.svc && p.svc.battery >= 0 && p.svc.battery <= 15 && !p.svc.charging ? "#f38ba8" : p.ink
                    font.pixelSize: 16; font.bold: true; font.family: p.mono
                }
                Rectangle {
                    anchors.right: parent.right
                    width: 64; height: 5; radius: 3; color: p.chip
                    Rectangle {
                        width: parent.width * Math.max(0, Math.min(100, p.svc ? p.svc.battery : 0)) / 100
                        height: parent.height; radius: 3
                        color: p.svc && p.svc.charging ? "#a6e3a1" : (p.svc && p.svc.battery <= 15 ? "#f38ba8" : p.accent)
                    }
                }
            }
        }

        // ---- noise control --------------------------------------------------
        Column {
            width: parent.width
            spacing: 8
            visible: p.caps.anc === true || p.caps.ambient === true
            Section { text: "Noise control" }
            Row {
                spacing: 8
                Chip { label: "Cancelling"; on: p.svc && p.svc.mode === "nc"; onClicked: p.svc.setNoise("nc") }
                Chip { label: "Ambient"; on: p.svc && p.svc.mode === "ambient"; onClicked: p.svc.setNoise("ambient") }
                Chip { label: "Off"; on: p.svc && p.svc.mode === "off"; onClicked: p.svc.setNoise("off") }
            }
            // Ambient level (1-20) + voice focus, only while in ambient mode.
            Column {
                width: parent.width
                spacing: 6
                visible: p.svc && p.svc.mode === "ambient"
                Item {
                    width: parent.width; height: 24
                    Text { anchors.verticalCenter: parent.verticalCenter; text: "Ambient level"; color: p.dim; font.pixelSize: 11; font.family: p.mono }
                    Text { anchors.verticalCenter: parent.verticalCenter; anchors.right: parent.right; text: p.svc ? p.svc.ambientLevel : ""; color: p.ink; font.pixelSize: 12; font.family: p.mono }
                }
                Item {
                    id: slider
                    width: parent.width; height: 20
                    property real frac: p.svc ? (p.svc.ambientLevel - 1) / 19 : 0
                    Rectangle { anchors.verticalCenter: parent.verticalCenter; width: parent.width; height: 4; radius: 2; color: p.chip }
                    Rectangle { anchors.verticalCenter: parent.verticalCenter; width: parent.width * slider.frac; height: 4; radius: 2; color: p.accent }
                    Rectangle {
                        anchors.verticalCenter: parent.verticalCenter
                        x: (parent.width - width) * slider.frac
                        width: 16; height: 16; radius: 8; color: "#ffffff"
                    }
                    MouseArea {
                        anchors.fill: parent
                        cursorShape: Qt.PointingHandCursor
                        function setFrom(mx) {
                            const f = Math.max(0, Math.min(1, mx / width));
                            const level = 1 + Math.round(f * 19);
                            if (p.svc && level !== p.svc.ambientLevel) p.svc.setAmbient(level, p.svc.focusOnVoice);
                        }
                        onReleased: mouse => setFrom(mouse.x)
                        onPositionChanged: mouse => { if (pressed) slider.frac = Math.max(0, Math.min(1, mouse.x / width)); }
                    }
                }
                ToggleRow {
                    width: parent.width
                    label: "Focus on voice"
                    on: p.svc && p.svc.focusOnVoice
                    onToggled: v => p.svc.setAmbient(p.svc.ambientLevel, v)
                }
            }
        }

        // ---- equalizer ------------------------------------------------------
        Column {
            width: parent.width
            spacing: 8
            visible: p.caps.equalizer === true
            Item {
                width: parent.width; height: 14
                Section { anchors.left: parent.left; text: "Equalizer" }
                Text {
                    anchors.right: parent.right
                    visible: p.caps.ult === true && p.svc && p.svc.ultMode >= 0
                    text: p.svc && p.svc.ultMode === 0 ? "ULT off" : "ULT " + (p.svc ? p.svc.ultMode : "")
                    color: p.svc && p.svc.ultMode > 0 ? p.accent : p.dim
                    font.pixelSize: 11; font.family: p.mono
                }
            }
            Flow {
                width: parent.width
                spacing: 6
                Repeater {
                    model: p.presets
                    delegate: Chip {
                        required property var modelData
                        label: modelData.label
                        on: p.svc && p.svc.eqPreset === modelData.id
                        onClicked: p.svc.setEq(modelData.id)
                    }
                }
            }
        }

        // ---- features -------------------------------------------------------
        Column {
            width: parent.width
            spacing: 2
            visible: p.caps.pauseWhenRemoved === true || p.caps.dsee === true
            Section { text: "Features" }
            ToggleRow {
                width: parent.width
                visible: p.caps.pauseWhenRemoved === true
                label: "Pause on removal"
                hint: "Pause music when you take them off"
                on: p.svc && p.svc.pauseWhenRemoved
                onToggled: v => p.svc.setPause(v)
            }
            ToggleRow {
                width: parent.width
                visible: p.caps.dsee === true
                label: "DSEE"
                hint: "Restores highs lost to compression"
                on: p.svc && p.svc.dsee
                onToggled: v => p.svc.setDsee(v)
            }
        }

        // ---- connected devices (multipoint) ---------------------------------
        Column {
            width: parent.width
            spacing: 6
            visible: p.caps.connectedDevices === true
            Section { text: "Connected devices" }
            Repeater {
                model: p.svc ? p.svc.devices : []
                delegate: Item {
                    required property var modelData
                    width: col.width; height: 26
                    Text {
                        anchors.verticalCenter: parent.verticalCenter
                        text: "  " + modelData.name
                        color: p.ink; font.pixelSize: 12; font.family: p.mono
                    }
                    Text {
                        anchors.verticalCenter: parent.verticalCenter
                        anchors.right: parent.right
                        visible: modelData.playback
                        text: "playing audio"; color: p.accent; font.pixelSize: 10; font.family: p.mono
                    }
                }
            }
            Text {
                visible: p.svc && p.svc.devices.length === 0
                text: p.svc && p.svc.devicesStatus === "unsupported"
                    ? "List unavailable — enable multipoint on the headset." : "No devices reported."
                color: p.dim; font.pixelSize: 11; font.family: p.mono
            }
        }

        Text {
            width: parent.width
            visible: p.svc && p.svc.error !== ""
            text: p.svc ? p.svc.error : ""
            color: "#f38ba8"; font.pixelSize: 11; font.family: p.mono
            wrapMode: Text.Wrap
        }
    }
}
