//! Read-only state dump for ULT WEAR (no writes are ever sent).
use sony_protocol::{DataType, Protocol, ProtocolV2, Session, SonyFrame};
use sony_transport::RfcommTransport;
use std::sync::Arc;
use std::time::Duration;

fn raw(s: &Session, req: &[u8], ret: u8, sub: i32) -> String {
    let f = SonyFrame { data_type: DataType::DataMdr, sequence: 0, payload: req.to_vec() };
    match s.send_and_await(&f, ret, sub, Duration::from_millis(1500)) {
        Ok(r) => r.payload.iter().map(|b| format!("{b:02x}")).collect::<Vec<_>>().join(" "),
        Err(e) => format!("{e}"),
    }
}

fn main() {
    let addr = std::env::args().nth(1).expect("usage: probe <MAC>");
    let t = Arc::new(RfcommTransport::new());
    let s = Arc::new(Session::new(t));
    s.connect(&addr).expect("connect");
    let p = ProtocolV2::new(s.clone(), false, true);
    let _ = p.init_device();
    println!("firmware     : {:?}", p.get_firmware_version());
    println!("codec        : {:?}", p.get_codec());
    println!("battery      : {:?}", p.get_battery());
    println!("noise control: {:?}", p.get_noise_control());
    println!("equalizer    : {:?}", p.get_equalizer());
    println!("pause removed: {:?}", p.get_pause_when_removed());
    println!("auto power   : {:?}", p.get_auto_power_off());
    println!("multipoint   : {}", raw(&s, &[0xd6, 0xd1], 0xd7, 0xd1));
    println!("touch panel  : {}", raw(&s, &[0xd6, 0xd2], 0xd7, 0xd2));
    println!("sidetone     : {}", raw(&s, &[0xd6, 0xd3], 0xd7, 0xd3));
    println!("auto power raw: {}", raw(&s, &[0x26, 0x05], 0x27, -1));
    println!("device list  : {:?}", p.get_connected_devices());
    s.disconnect();
}
