#[cxx_qt::bridge]
pub mod qobject {
    unsafe extern "RustQt" {
        #[qobject]
        #[qml_element]
        #[qml_singleton]
        #[qproperty(i32, battery_level, cxx_name = "batteryLevel")]
        type DeviceController = super::DeviceControllerRust;

        #[qinvokable]
        #[cxx_name = "bump"]
        fn bump(self: Pin<&mut DeviceController>);
    }
}

use core::pin::Pin;

#[derive(Default)]
pub struct DeviceControllerRust {
    battery_level: i32,
}

impl qobject::DeviceController {
    pub fn bump(self: Pin<&mut Self>) {
        let v = *self.battery_level() + 1;
        self.set_battery_level(v);
    }
}
