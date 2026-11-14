use cxx_qt_build::{CxxQtBuilder, QmlModule};

fn main() {
    CxxQtBuilder::new_qml_module(
        QmlModule::new("SonyCenter").qml_file("qml/Main.qml"),
    )
    .qt_module("Quick")
    .qt_module("QuickControls2")
    .file("src/controller.rs")
    .build();
}
