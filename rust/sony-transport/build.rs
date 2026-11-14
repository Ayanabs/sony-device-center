fn main() {
    // SDP service lookup goes through BlueZ's libbluetooth (bluez-libs).
    println!("cargo:rustc-link-lib=bluetooth");
}
