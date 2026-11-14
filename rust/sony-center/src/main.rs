mod backend;
mod controller;
mod device_core;
mod i18n;

use cxx_qt_lib::{QGuiApplication, QQmlApplicationEngine, QString, QUrl};

fn main() {
    // Main.qml customises control backgrounds/handles; only the "Basic" style
    // honours that on every platform (the native styles warn per control).
    std::env::set_var("QT_QUICK_CONTROLS_STYLE", "Basic");

    let mut app = QGuiApplication::new();
    if let Some(mut app) = app.as_mut() {
        app.as_mut().set_application_name(&QString::from("Sony Device Center"));
        app.as_mut().set_organization_name(&QString::from("SonyBridge"));
        // Wayland shells match the window to its .desktop entry by this name.
        QGuiApplication::set_desktop_file_name(&QString::from("sony-device-center"));
    }

    let mut engine = QQmlApplicationEngine::new();
    if let Some(engine) = engine.as_mut() {
        engine.load(&QUrl::from("qrc:/qt/qml/SonyCenter/qml/Main.qml"));
    }
    if let Some(app) = app.as_mut() {
        app.exec();
    }
}
