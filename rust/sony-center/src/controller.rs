//! cxx-qt controller: the QML-facing `DeviceController` singleton. It mirrors the
//! C++ `DeviceCenterController` property-for-property so `Main.qml` runs unchanged.
//! (Generated section: properties and invokable declarations.)

#[cxx_qt::bridge]
pub mod qobject {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
        include!("cxx-qt-lib/qvariant.h");
        type QVariant = cxx_qt_lib::QVariant;
        include!("cxx-qt-lib/qlist.h");
        type QList_QVariant = cxx_qt_lib::QList<QVariant>;
        include!("cxx-qt-lib/qmap.h");
        type QMap_QString_QVariant = cxx_qt_lib::QMap<cxx_qt_lib::QMapPair_QString_QVariant>;
    }

    unsafe extern "RustQt" {
        #[qobject]
        #[qml_element]
        #[qml_singleton]
        #[qproperty(bool, busy, cxx_name = "busy")]
        #[qproperty(QString, last_error, cxx_name = "lastError")]
        #[qproperty(QString, connection_state, cxx_name = "connectionState")]
        #[qproperty(QString, codec, cxx_name = "codec")]
        #[qproperty(QMap_QString_QVariant, feature_status, cxx_name = "featureStatus")]
        #[qproperty(QString, device_name, cxx_name = "deviceName")]
        #[qproperty(QString, device_address, cxx_name = "deviceAddress")]
        #[qproperty(bool, connected, cxx_name = "connected")]
        #[qproperty(i32, battery_level, cxx_name = "batteryLevel")]
        #[qproperty(bool, is_charging, cxx_name = "isCharging")]
        #[qproperty(QString, noise_control_mode, cxx_name = "noiseControlMode")]
        #[qproperty(i32, ambient_level, cxx_name = "ambientLevel")]
        #[qproperty(bool, focus_on_voice, cxx_name = "focusOnVoice")]
        #[qproperty(i32, equalizer_preset, cxx_name = "equalizerPreset", READ, NOTIFY)]
        #[qproperty(QString, equalizer_preset_name, cxx_name = "equalizerPresetName")]
        #[qproperty(i32, clear_bass, cxx_name = "clearBass")]
        #[qproperty(QList_QVariant, equalizer_bands, cxx_name = "equalizerBands")]
        #[qproperty(i32, ult_mode, cxx_name = "ultMode")]
        #[qproperty(bool, has_ult, cxx_name = "hasUlt")]
        #[qproperty(bool, pause_when_removed, cxx_name = "pauseWhenRemoved", READ, NOTIFY)]
        #[qproperty(bool, has_pause_when_removed, cxx_name = "hasPauseWhenRemoved")]
        #[qproperty(bool, dsee, cxx_name = "dsee", READ, NOTIFY)]
        #[qproperty(bool, speak_to_chat, cxx_name = "speakToChat", READ, NOTIFY)]
        #[qproperty(bool, adaptive_volume, cxx_name = "adaptiveVolume", READ, NOTIFY)]
        #[qproperty(i32, auto_power_off, cxx_name = "autoPowerOff", READ, NOTIFY)]
        #[qproperty(QString, hero_image_path, cxx_name = "heroImagePath")]
        #[qproperty(bool, has_anc, cxx_name = "hasAnc")]
        #[qproperty(bool, has_ambient, cxx_name = "hasAmbient")]
        #[qproperty(bool, has_equalizer, cxx_name = "hasEqualizer")]
        #[qproperty(bool, has_clear_bass, cxx_name = "hasClearBass")]
        #[qproperty(bool, ten_band_equalizer, cxx_name = "tenBandEqualizer")]
        #[qproperty(bool, has_dsee, cxx_name = "hasDsee")]
        #[qproperty(bool, has_speak_to_chat, cxx_name = "hasSpeakToChat")]
        #[qproperty(bool, has_adaptive_volume, cxx_name = "hasAdaptiveVolume")]
        #[qproperty(QList_QVariant, paired_devices, cxx_name = "pairedDevices")]
        #[qproperty(QList_QVariant, connected_devices, cxx_name = "connectedDevices")]
        #[qproperty(QString, connected_devices_status, cxx_name = "connectedDevicesStatus")]
        #[qproperty(bool, has_connected_devices, cxx_name = "hasConnectedDevices")]
        #[qproperty(bool, autostart, cxx_name = "autostart", READ, NOTIFY)]
        #[qproperty(QString, app_version, cxx_name = "appVersion")]
        #[qproperty(QString, current_language, cxx_name = "currentLanguage")]
        #[qproperty(QList_QVariant, available_languages, cxx_name = "availableLanguages")]
        type DeviceController = super::DeviceControllerRust;

        /// Fired after every state change (the C++ controller's `stateChanged`);
        /// `Main.qml` listens to it to re-sync switch positions.
        #[qsignal]
        #[cxx_name = "stateChanged"]
        fn state_changed(self: Pin<&mut DeviceController>);

        #[qinvokable]
        #[cxx_name = "setAnc"]
        fn set_anc(self: Pin<&mut DeviceController>, enabled: bool);

        #[qinvokable]
        #[cxx_name = "setAmbient"]
        fn set_ambient(self: Pin<&mut DeviceController>, level: i32, focus_on_voice: bool);

        #[qinvokable]
        #[cxx_name = "setNoiseControlOff"]
        fn set_noise_control_off(self: Pin<&mut DeviceController>);

        #[qinvokable]
        #[cxx_name = "setEqualizerPreset"]
        fn set_equalizer_preset(self: Pin<&mut DeviceController>, preset: i32);

        #[qinvokable]
        #[cxx_name = "setEqualizerCustom"]
        fn set_equalizer_custom(self: Pin<&mut DeviceController>, clear_bass: i32, bands: &QList_QVariant);

        #[qinvokable]
        #[cxx_name = "setPauseWhenRemoved"]
        fn set_pause_when_removed(self: Pin<&mut DeviceController>, enabled: bool);

        #[qinvokable]
        #[cxx_name = "setDsee"]
        fn set_dsee(self: Pin<&mut DeviceController>, enabled: bool);

        #[qinvokable]
        #[cxx_name = "setSpeakToChat"]
        fn set_speak_to_chat(self: Pin<&mut DeviceController>, enabled: bool);

        #[qinvokable]
        #[cxx_name = "setAdaptiveVolume"]
        fn set_adaptive_volume(self: Pin<&mut DeviceController>, enabled: bool);

        #[qinvokable]
        #[cxx_name = "setAutoPowerOff"]
        fn set_auto_power_off(self: Pin<&mut DeviceController>, index: i32);

        #[qinvokable]
        #[cxx_name = "setAutostart"]
        fn set_autostart_enabled(self: Pin<&mut DeviceController>, enable: bool);

        #[qinvokable]
        #[cxx_name = "setLanguage"]
        fn set_language(self: Pin<&mut DeviceController>, code: &QString);

        #[qinvokable]
        #[cxx_name = "t"]
        fn t(self: Pin<&mut DeviceController>, key: &QString) -> QString;

        #[qinvokable]
        #[cxx_name = "openUrl"]
        fn open_url(self: Pin<&mut DeviceController>, url: &QString);

        #[qinvokable]
        #[cxx_name = "connectDevice"]
        fn connect_device(self: Pin<&mut DeviceController>, address: &QString, name: &QString);

        #[qinvokable]
        #[cxx_name = "disconnectDevice"]
        fn disconnect_device(self: Pin<&mut DeviceController>);

        #[qinvokable]
        #[cxx_name = "refreshDiscoveredDevices"]
        fn refresh_discovered_devices(self: Pin<&mut DeviceController>);

        #[qinvokable]
        #[cxx_name = "clearError"]
        fn clear_error(self: Pin<&mut DeviceController>);

        #[qinvokable]
        #[cxx_name = "start"]
        fn start(self: Pin<&mut DeviceController>);
    }

    impl cxx_qt::Threading for DeviceController {}
}

pub use qobject::DeviceController;
pub(crate) use qobject::{QList_QVariant, QMap_QString_QVariant};

use sony_app::{Backend, Emit, PairedDevice, Request, TransportFactory, Update};
use sony_app::{Command, Snapshot};
use crate::i18n;
use core::pin::Pin;
use cxx_qt::{CxxQtType, Threading};
use cxx_qt_lib::{QList, QMap, QMapPair_QString_QVariant, QString, QVariant};
use std::sync::Arc;

pub struct DeviceControllerRust {
    busy: bool,
    last_error: QString,
    connection_state: QString,
    codec: QString,
    feature_status: QMap<QMapPair_QString_QVariant>,
    device_name: QString,
    device_address: QString,
    connected: bool,
    battery_level: i32,
    is_charging: bool,
    noise_control_mode: QString,
    ambient_level: i32,
    focus_on_voice: bool,
    equalizer_preset: i32,
    equalizer_preset_name: QString,
    clear_bass: i32,
    equalizer_bands: QList<QVariant>,
    ult_mode: i32,
    has_ult: bool,
    pause_when_removed: bool,
    has_pause_when_removed: bool,
    dsee: bool,
    speak_to_chat: bool,
    adaptive_volume: bool,
    auto_power_off: i32,
    hero_image_path: QString,
    has_anc: bool,
    has_ambient: bool,
    has_equalizer: bool,
    has_clear_bass: bool,
    ten_band_equalizer: bool,
    has_dsee: bool,
    has_speak_to_chat: bool,
    has_adaptive_volume: bool,
    paired_devices: QList<QVariant>,
    connected_devices: QList<QVariant>,
    connected_devices_status: QString,
    has_connected_devices: bool,
    autostart: bool,
    app_version: QString,
    current_language: QString,
    available_languages: QList<QVariant>,
    backend: Option<Backend>,
}

impl Default for DeviceControllerRust {
    fn default() -> Self {
        Self {
            busy: true,
            last_error: QString::default(),
            connection_state: QString::from("searching"),
            codec: QString::from("Unknown"),
            feature_status: QMap::default(),
            device_name: QString::default(),
            device_address: QString::default(),
            connected: false,
            battery_level: -1,
            is_charging: false,
            noise_control_mode: QString::from("unknown"),
            ambient_level: 10,
            focus_on_voice: false,
            equalizer_preset: 0,
            equalizer_preset_name: QString::from("Off"),
            clear_bass: 0,
            equalizer_bands: QList::default(),
            ult_mode: -1,
            has_ult: false,
            pause_when_removed: false,
            has_pause_when_removed: false,
            dsee: false,
            speak_to_chat: false,
            adaptive_volume: false,
            auto_power_off: 0,
            hero_image_path: QString::from("resources/devices/wh-1000xm5.png"),
            has_anc: false,
            has_ambient: false,
            has_equalizer: false,
            has_clear_bass: false,
            ten_band_equalizer: false,
            has_dsee: false,
            has_speak_to_chat: false,
            has_adaptive_volume: false,
            paired_devices: QList::default(),
            connected_devices: QList::default(),
            connected_devices_status: QString::from("unknown"),
            has_connected_devices: false,
            autostart: false,
            app_version: QString::from(env!("CARGO_PKG_VERSION")),
            current_language: QString::from("en"),
            available_languages: QList::default(),
            backend: None,
        }
    }
}

// ---------------------------------------------------------------------------
// Hand-written section
// ---------------------------------------------------------------------------

fn vmap(entries: Vec<(&str, QVariant)>) -> QMap<QMapPair_QString_QVariant> {
    let mut m = QMap::default();
    for (k, v) in entries {
        m.insert(QString::from(k), v);
    }
    m
}

fn vlist(items: Vec<QVariant>) -> QList<QVariant> {
    let mut l = QList::default();
    for i in items {
        l.append(i);
    }
    l
}

fn qv_str(s: &str) -> QVariant {
    QVariant::from(&QString::from(s))
}

fn settings_path() -> std::path::PathBuf {
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(std::path::PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| std::path::PathBuf::from(h).join(".config")))
        .unwrap_or_default();
    base.join("SonyBridge/SonyDeviceCenter.conf")
}

/// Same ini file Qt's `QSettings("SonyBridge", "SonyDeviceCenter")` uses, so the
/// C++ and Rust apps share the language choice.
fn load_language() -> String {
    std::fs::read_to_string(settings_path())
        .ok()
        .and_then(|t| t.lines().find_map(|l| l.strip_prefix("language=").map(str::to_owned)))
        .unwrap_or_else(|| "en".into())
}

fn save_language(code: &str) {
    let path = settings_path();
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let _ = std::fs::write(path, format!("[General]\nlanguage={code}\n"));
}

fn autostart_file() -> std::path::PathBuf {
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(std::path::PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| std::path::PathBuf::from(h).join(".config")))
        .unwrap_or_default();
    base.join("autostart/sony-device-center.desktop")
}

fn hero_image_path(name: &str) -> &'static str {
    let l = name.to_lowercase();
    let wf = l.contains("wf");
    match () {
        _ if wf && l.contains("xm6") => "resources/devices/wf-1000xm6.png",
        _ if wf && l.contains("xm5") => "resources/devices/wf-1000xm5.png",
        _ if wf && l.contains("xm4") => "resources/devices/wf-1000xm4.png",
        _ if wf && l.contains("xm3") => "resources/devices/wf-1000xm3.png",
        _ if l.contains("xm6") => "resources/devices/wh-1000xm6.png",
        _ if l.contains("xm5") => "resources/devices/wh-1000xm5.png",
        _ if l.contains("xm4") => "resources/devices/wh-1000xm4.png",
        _ if l.contains("xm3") => "resources/devices/wh-1000xm3.png",
        _ if l.contains("ch720") => "resources/devices/wh-ch720n.png",
        _ if l.contains("linkbuds") => "resources/devices/linkbuds-s.png",
        _ => "resources/devices/wh-1000xm5.png",
    }
}

/// Updates a read-only (to QML) property and emits its change signal when it differs.
macro_rules! ro {
    ($s:ident, $field:ident, $sig:ident, $v:expr) => {{
        let v = $v;
        if $s.rust().$field != v {
            $s.as_mut().rust_mut().$field = v;
            $s.as_mut().$sig();
        }
    }};
}

fn to_i32(v: &QVariant) -> i32 {
    v.value::<i32>().or_else(|| v.value::<f64>().map(|f| f.round() as i32)).unwrap_or(0)
}

impl qobject::DeviceController {
    fn send(mut self: Pin<&mut Self>, req: Request) {
        if *self.busy() {
            return;
        }
        self.as_mut().set_busy(true);
        self.as_mut().set_last_error(QString::default());
        if let Some(b) = self.rust().backend.as_ref() {
            b.request(req);
        }
    }

    fn cmd(self: Pin<&mut Self>, c: Command) {
        self.send(Request::Command(c));
    }

    fn apply_update(mut self: Pin<&mut Self>, u: Update) {
        self.as_mut().apply_update_inner(u);
        self.as_mut().state_changed();
    }

    fn apply_update_inner(mut self: Pin<&mut Self>, u: Update) {
        match u {
            Update::Snapshot(s) => self.as_mut().apply_snapshot(&s),
            Update::Devices(devs) => {
                let list = vlist(
                    devs.iter()
                        .map(|PairedDevice { name, address }| {
                            QVariant::from(&vmap(vec![("name", qv_str(name)), ("address", qv_str(address))]))
                        })
                        .collect(),
                );
                self.as_mut().set_paired_devices(list);
            }
            Update::Error(e) => self.as_mut().set_last_error(QString::from(e.as_str())),
            Update::Completed => self.as_mut().set_busy(false),
        }
    }

    fn apply_snapshot(mut self: Pin<&mut Self>, s: &Snapshot) {
        self.as_mut().set_connected(s.connected);
        self.as_mut().set_connection_state(QString::from(s.connection_state.as_str()));
        if !s.name.is_empty() || !s.connected {
            self.as_mut().set_device_name(QString::from(s.name.as_str()));
            self.as_mut().set_hero_image_path(QString::from(hero_image_path(&s.name)));
        }
        if !s.address.is_empty() || !s.connected {
            self.as_mut().set_device_address(QString::from(s.address.as_str()));
        }
        let features = vmap(
            s.features
                .iter()
                .map(|(k, v)| {
                    (k.as_str(), QVariant::from(&vmap(vec![("availability", qv_str(v))])))
                })
                .collect(),
        );
        self.as_mut().set_feature_status(features);

        let c = &s.caps;
        self.as_mut().set_has_anc(c.anc);
        self.as_mut().set_has_ambient(c.ambient);
        self.as_mut().set_has_equalizer(c.equalizer);
        self.as_mut().set_has_clear_bass(c.clear_bass);
        self.as_mut().set_ten_band_equalizer(c.ten_band_equalizer);
        self.as_mut().set_has_dsee(c.dsee);
        self.as_mut().set_has_speak_to_chat(c.speak_to_chat);
        self.as_mut().set_has_adaptive_volume(c.adaptive_volume);
        self.as_mut().set_has_ult(c.ult_bass);
        self.as_mut().set_has_pause_when_removed(c.pause_when_removed);
        self.as_mut().set_has_connected_devices(c.connected_devices);

        self.as_mut().set_battery_level(s.battery);
        self.as_mut().set_is_charging(s.connected && s.charging);
        self.as_mut().set_noise_control_mode(QString::from(s.noise_mode.as_str()));
        self.as_mut().set_ambient_level(s.ambient_level);
        self.as_mut().set_focus_on_voice(s.focus_on_voice);
        ro!(self, equalizer_preset, equalizer_preset_changed, s.eq_preset);
        self.as_mut().set_equalizer_preset_name(QString::from(s.eq_preset_name.as_str()));
        self.as_mut().set_clear_bass(s.clear_bass);
        self.as_mut().set_equalizer_bands(vlist(s.eq_bands.iter().map(|&b| QVariant::from(&b)).collect()));
        self.as_mut().set_ult_mode(s.ult_mode);
        ro!(self, dsee, dsee_changed, s.dsee);
        ro!(self, speak_to_chat, speak_to_chat_changed, s.speak_to_chat);
        ro!(self, adaptive_volume, adaptive_volume_changed, s.adaptive_volume);
        ro!(self, pause_when_removed, pause_when_removed_changed, s.pause_when_removed);
        ro!(self, auto_power_off, auto_power_off_changed, s.auto_power_off);
        self.as_mut().set_codec(QString::from(s.codec.as_str()));
        self.as_mut().set_connected_devices_status(QString::from(s.connected_devices_status.as_str()));
        self.as_mut().set_connected_devices(vlist(
            s.connected_devices
                .iter()
                .map(|d| {
                    QVariant::from(&vmap(vec![
                        ("address", qv_str(&d.address)),
                        ("name", qv_str(&d.name)),
                        ("playbackDevice", QVariant::from(&d.playback_device)),
                    ]))
                })
                .collect(),
        ));
        // The sets above leave `busy` alone: Update::Completed clears it.
    }

    pub fn start(mut self: Pin<&mut Self>) {
        if self.rust().backend.is_some() {
            return;
        }
        let lang = load_language();
        self.as_mut().set_current_language(QString::from(lang.as_str()));
        ro!(self, autostart, autostart_changed, autostart_file().exists());
        self.as_mut().set_available_languages(vlist(
            i18n::LANGUAGES
                .iter()
                .map(|(code, name)| {
                    QVariant::from(&vmap(vec![("code", qv_str(code)), ("name", qv_str(name))]))
                })
                .collect(),
        ));
        let qt = self.qt_thread();
        let emit: Emit = Arc::new(move |u| {
            let _ = qt.queue(move |obj| obj.apply_update(u));
        });
        let factory: TransportFactory = Arc::new(|| {
            Arc::new(sony_transport::RfcommTransport::new()) as Arc<dyn sony_protocol::Transport>
        });
        self.as_mut().rust_mut().backend = Some(Backend::start(factory, emit, true));
    }

    pub fn set_anc(self: Pin<&mut Self>, enabled: bool) {
        self.cmd(Command::Anc(enabled));
    }
    pub fn set_ambient(self: Pin<&mut Self>, level: i32, focus_on_voice: bool) {
        self.cmd(Command::Ambient { level, focus_on_voice });
    }
    pub fn set_noise_control_off(self: Pin<&mut Self>) {
        self.cmd(Command::Anc(false));
    }
    pub fn set_equalizer_preset(self: Pin<&mut Self>, preset: i32) {
        self.cmd(Command::EqPreset(preset));
    }
    pub fn set_equalizer_custom(self: Pin<&mut Self>, clear_bass: i32, bands: &QList_QVariant) {
        let bands = bands.iter().map(to_i32).collect();
        self.cmd(Command::EqCustom { clear_bass, bands });
    }
    pub fn set_pause_when_removed(self: Pin<&mut Self>, enabled: bool) {
        self.cmd(Command::PauseWhenRemoved(enabled));
    }
    pub fn set_dsee(self: Pin<&mut Self>, enabled: bool) {
        self.cmd(Command::Dsee(enabled));
    }
    pub fn set_speak_to_chat(self: Pin<&mut Self>, enabled: bool) {
        self.cmd(Command::SpeakToChat(enabled));
    }
    pub fn set_adaptive_volume(self: Pin<&mut Self>, enabled: bool) {
        self.cmd(Command::AdaptiveVolume(enabled));
    }
    pub fn set_auto_power_off(self: Pin<&mut Self>, index: i32) {
        self.cmd(Command::AutoPowerOff(index));
    }

    pub fn set_autostart_enabled(mut self: Pin<&mut Self>, enable: bool) {
        let file = autostart_file();
        if enable {
            if let Some(dir) = file.parent() {
                let _ = std::fs::create_dir_all(dir);
            }
            let exe = std::env::current_exe().map(|p| p.display().to_string()).unwrap_or_default();
            let _ = std::fs::write(
                &file,
                format!(
                    "[Desktop Entry]\nType=Application\nName=Sony Device Center\n\
                     Comment=Unofficial open-source companion for Sony WH/WF/LinkBuds audio devices\n\
                     Exec={exe}\nIcon=sony-device-center\nTerminal=false\n\
                     Categories=Audio;AudioVideo;Settings;\nX-GNOME-Autostart-enabled=true\n"
                ),
            );
        } else {
            let _ = std::fs::remove_file(&file);
        }
        ro!(self, autostart, autostart_changed, file.exists());
    }

    pub fn set_language(mut self: Pin<&mut Self>, code: &QString) {
        let code = String::from(code);
        if code != String::from(self.current_language()) {
            save_language(&code);
            self.as_mut().set_current_language(QString::from(code.as_str()));
        }
    }

    pub fn t(self: Pin<&mut Self>, key: &QString) -> QString {
        let lang = String::from(self.current_language());
        QString::from(i18n::translate(&String::from(key), &lang).as_str())
    }

    pub fn open_url(self: Pin<&mut Self>, url: &QString) {
        let url = String::from(url);
        if url.starts_with("https://") || url.starts_with("http://") {
            let _ = std::process::Command::new("xdg-open").arg(url).spawn();
        }
    }

    pub fn connect_device(self: Pin<&mut Self>, address: &QString, name: &QString) {
        self.send(Request::Connect { address: String::from(address), name: String::from(name) });
    }
    pub fn disconnect_device(self: Pin<&mut Self>) {
        self.send(Request::Disconnect);
    }
    pub fn refresh_discovered_devices(self: Pin<&mut Self>) {
        if let Some(b) = self.rust().backend.as_ref() {
            b.request(Request::Discover);
        }
    }
    pub fn clear_error(self: Pin<&mut Self>) {
        self.set_last_error(QString::default());
    }
}
