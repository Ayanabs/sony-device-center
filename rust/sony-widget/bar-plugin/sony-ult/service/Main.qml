import QtQuick
import Quickshell
import Quickshell.Io

// Owns the one headset connection: runs `sony-widget`, which prints a JSON line
// whenever anything changes (push-based, ~3 MB), and relays commands to it.
Item {
    id: svc
    property var pluginApi: null

    readonly property string bin: Quickshell.env("HOME") + "/.local/bin/sony-widget"

    property bool connected: false
    property string name: ""
    property int battery: -1
    property bool charging: false
    property string mode: "off"            // nc | ambient | off
    property int ambientLevel: 10
    property bool focusOnVoice: false
    property int eqPreset: -1
    property string eqPresetName: ""
    property int ultMode: -1
    property string codec: ""
    property string firmware: ""
    property bool dsee: false
    property bool pauseWhenRemoved: false
    property var caps: ({})
    property var devices: []
    property string devicesStatus: "unknown"
    property string error: ""

    function apply(line) {
        let o;
        try { o = JSON.parse(line); } catch (e) { return; }
        const s = o.state || {};
        connected = s.connected === true;
        error = s.error || "";
        if (!connected) { battery = -1; return; }
        name = s.name || "";
        battery = s.battery !== undefined ? s.battery : -1;
        charging = !!s.charging;
        mode = s.noise === "cancelling" ? "nc" : s.noise === "ambient" ? "ambient" : "off";
        ambientLevel = s.ambientLevel || 10;
        focusOnVoice = !!s.focusOnVoice;
        eqPreset = s.eqPreset !== undefined ? s.eqPreset : -1;
        eqPresetName = s.eqPresetName || "";
        ultMode = s.ultMode !== undefined ? s.ultMode : -1;
        codec = s.codec || "";
        firmware = s.firmware || "";
        dsee = !!s.dsee;
        pauseWhenRemoved = !!s.pauseWhenRemoved;
        caps = s.caps || ({});
        devices = s.connectedDevices || [];
        devicesStatus = s.connectedDevicesStatus || "unknown";
    }

    function run(args) { action.command = [bin].concat(args); action.running = true; }
    function cycleNoiseControl() { run(["cycle-nc"]); }
    function noiseControlOff() { run(["set-nc", "off"]); }
    function setNoise(m) { run(["set-nc", m]); }
    function setAmbient(level, voice) { run(["set-ambient", String(level), voice ? "on" : "off"]); }
    function setEq(id) { run(["set-eq", String(id)]); }
    function setPause(on) { run(["set-pause", on ? "on" : "off"]); }
    function setDsee(on) { run(["set-dsee", on ? "on" : "off"]); }

    Process {
        id: feed
        running: true
        command: [svc.bin]
        stdout: SplitParser { onRead: data => svc.apply(data) }
        onExited: restart.start()
    }
    // Stop the child when the shell unloads / reloads this plugin, so reloads never leave a stray widget behind.
    Component.onDestruction: { restart.stop(); feed.running = false }
    Timer { id: restart; interval: 3000; onTriggered: feed.running = true }
    Process { id: action }
}
