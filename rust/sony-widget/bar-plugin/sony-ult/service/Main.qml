import QtQuick
import Quickshell
import Quickshell.Io

// Owns the one headset connection: runs `sony-widget`, which prints a JSON line
// whenever battery / noise-control / connection changes (push-based, ~3 MB).
Item {
    id: svc
    property var pluginApi: null

    readonly property string bin: Quickshell.env("HOME") + "/.local/bin/sony-widget"

    property bool connected: false
    property int battery: -1
    property bool charging: false
    property string mode: "off"      // nc | ambient | off
    property string tip: "Headset not connected"

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

    function run(args) { action.command = [bin].concat(args); action.running = true; }
    function cycleNoiseControl() { run(["cycle-nc"]); }
    function noiseControlOff() { run(["set-nc", "off"]); }

    Process {
        id: feed
        running: true
        command: [svc.bin]
        stdout: SplitParser { onRead: data => svc.apply(data) }
        onExited: restart.start()
    }
    Timer { id: restart; interval: 3000; onTriggered: feed.running = true }
    Process { id: action }
}
