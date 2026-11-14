use cxx_qt_build::{CxxQtBuilder, QmlModule};
use std::{env, fs, path::PathBuf};

/// The C++ app's `Main.qml` is the single UI source. It talks to a context
/// property called `controller`; here that is the `DeviceController` singleton,
/// so we generate an adapted copy: import the module, expose the singleton as
/// `controller`, start the backend, and load hero images from the Qt resources.
fn adapt_qml(src: &str) -> String {
    let mut out = String::with_capacity(src.len() + 256);
    let mut injected_import = false;
    let mut injected_root = false;
    for line in src.lines() {
        out.push_str(line);
        out.push('\n');
        if !injected_import && line.starts_with("import ") && line.contains("QtQuick.Shapes") {
            out.push_str("import SonyCenter\n");
            injected_import = true;
        }
        if !injected_root && line.trim_start().starts_with("ApplicationWindow {") {
            out.push_str("    readonly property var controller: DeviceController\n");
            out.push_str("    Component.onCompleted: DeviceController.start()\n");
            injected_root = true;
        }
    }
    assert!(injected_import && injected_root, "Main.qml layout changed: update adapt_qml()");
    let patched = out.replace(
        "source: \"../\" + controller.heroImagePath",
        "source: \"qrc:/\" + controller.heroImagePath",
    );
    assert!(patched.contains("qrc:/\" + controller.heroImagePath"), "hero image source not found");
    // `a && b && c` can yield `undefined` (not `false`) when the map lookup misses.
    let known = "readonly property bool known: controller.connected && availability && availability.availability === \"valid\"";
    assert!(patched.contains(known), "feature-card binding not found");
    patched.replace(known, "readonly property bool known: !!(controller.connected && availability && availability.availability === \"valid\")")
}

fn main() {
    let manifest = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let src_qml = manifest.join("../../apps/device-center/qml/Main.qml");
    println!("cargo::rerun-if-changed={}", src_qml.display());
    println!("cargo::rerun-if-changed=build.rs");

    let qml_dir = manifest.join("qml");
    fs::create_dir_all(&qml_dir).unwrap();
    let adapted = adapt_qml(&fs::read_to_string(&src_qml).expect("read Main.qml"));
    fs::write(qml_dir.join("Main.qml"), adapted).unwrap();

    // Device hero images -> qrc:/resources/devices/<name>.png
    let img_dir = manifest.join("../../Client/resources/devices").canonicalize().unwrap();
    let out = PathBuf::from(env::var("OUT_DIR").unwrap());
    let mut qrc = String::from("<RCC>\n  <qresource prefix=\"/resources/devices\">\n");
    for e in fs::read_dir(&img_dir).unwrap().flatten() {
        let p = e.path();
        if p.extension().is_some_and(|x| x == "png") {
            println!("cargo::rerun-if-changed={}", p.display());
            qrc.push_str(&format!(
                "    <file alias=\"{}\">{}</file>\n",
                p.file_name().unwrap().to_string_lossy(),
                p.display()
            ));
        }
    }
    qrc.push_str("  </qresource>\n</RCC>\n");
    let qrc_path = out.join("devices.qrc");
    fs::write(&qrc_path, qrc).unwrap();

    CxxQtBuilder::new_qml_module(QmlModule::new("SonyCenter").qml_file("qml/Main.qml"))
        .qt_module("Quick")
        .qt_module("QuickControls2")
        .qrc(&qrc_path)
        .file("src/controller.rs")
        .build();
}
