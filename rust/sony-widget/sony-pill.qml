import QtQuick
import Quickshell
import Quickshell.Io

// Sony headphone pill. Reads JSON lines from `sony-widget` (push-based, so it
// idles at ~0% CPU) and shows battery + noise-control mode. Hidden while the
// headset is disconnected.
//   left click  -> cycle NC / Ambient / Off
//   right click -> noise control off
ShellRoot {
    id: root

    readonly property string bin: Quickshell.env("HOME") + "/.local/bin/sony-widget"

    property bool connected: false
    property int battery: -1
    property bool charging: false
    property string mode: "off"   // nc | ambient | off
    property string tip: ""

    function apply(line) {
        let o;
        try { o = JSON.parse(line); } catch (e) { return; }
        const cls = Array.isArray(o.class) ? o.class : [o.class];
        connected = !cls.includes("disconnected");
        charging = cls.includes("charging");
        mode = cls.includes("nc") ? "nc" : cls.includes("ambient") ? "ambient" : "off";
        battery = connected && o.percentage !== undefined ? o.percentage : -1;
        tip = o.tooltip || "";
    }

    Process {
        id: feed
        running: true
        command: [root.bin]
        stdout: SplitParser { onRead: data => root.apply(data) }
        // Restart if it ever exits (e.g. binary replaced during an upgrade).
        onExited: restart.start()
    }
    Timer { id: restart; interval: 3000; onTriggered: feed.running = true }

    Process { id: action }
    function run(args) { action.command = [bin].concat(args); action.running = true; }

    PanelWindow {
        id: win
        visible: root.connected
        anchors { top: true; right: true }
        margins { top: 8; right: 12 }
        implicitWidth: pill.width
        implicitHeight: pill.height
        color: "transparent"
        exclusionMode: ExclusionMode.Ignore

        Rectangle {
            id: pill
            width: row.implicitWidth + 24
            height: 32
            radius: 16
            color: "#cc1e1e2e"
            border.width: 1
            border.color: root.battery >= 0 && root.battery <= 15 && !root.charging ? "#f38ba8" : "#33ffffff"

            Row {
                id: row
                anchors.centerIn: parent
                spacing: 8
                Text {
                    anchors.verticalCenter: parent.verticalCenter
                    color: root.mode === "nc" ? "#89b4fa" : root.mode === "ambient" ? "#f9e2af" : "#a6adc8"
                    font.pixelSize: 13
                    font.bold: true
                    text: root.mode === "nc" ? "NC" : root.mode === "ambient" ? "AMB" : "OFF"
                }
                Text {
                    anchors.verticalCenter: parent.verticalCenter
                    visible: root.battery >= 0
                    color: root.charging ? "#a6e3a1" : root.battery <= 15 ? "#f38ba8" : "#cdd6f4"
                    font.pixelSize: 13
                    text: root.battery + "%" + (root.charging ? " +" : "")
                }
            }

            MouseArea {
                anchors.fill: parent
                acceptedButtons: Qt.LeftButton | Qt.RightButton
                hoverEnabled: true
                cursorShape: Qt.PointingHandCursor
                onClicked: mouse => root.run(mouse.button === Qt.RightButton
                    ? ["set-nc", "off"] : ["cycle-nc"])
            }
        }
    }
}
