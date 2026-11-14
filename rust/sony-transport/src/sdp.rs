//! SDP lookup of the Sony RFCOMM channel via libbluetooth.

use super::parse_bdaddr;
use std::os::raw::{c_int, c_void};
use std::ptr;

/// Which Sony service generation answered the SDP query.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProtocolKind {
    V1,
    V2,
}

pub const SERVICE_UUID_V1: [u8; 16] = [
    0x96, 0xcc, 0x20, 0x3e, 0x50, 0x68, 0x46, 0xad, 0xb3, 0x2d, 0xe3, 0x16, 0xf5, 0xe0, 0x69, 0xba,
];
pub const SERVICE_UUID_V2: [u8; 16] = [
    0x95, 0x6c, 0x7b, 0x26, 0xd4, 0x9a, 0x4b, 0xa8, 0xb0, 0x3f, 0xb1, 0x7d, 0x39, 0x3c, 0xb6, 0xe2,
];

const SDP_RETRY_IF_BUSY: u32 = 0x01;
const SDP_ATTR_REQ_RANGE: c_int = 2;
const RFCOMM_UUID: c_int = 0x0003;

#[repr(C)]
struct SdpList {
    next: *mut SdpList,
    data: *mut c_void,
}

/// `uuid_t`: u8 type, 3 bytes padding, 16-byte value union.
#[repr(C)]
struct Uuid {
    kind: u8,
    _pad: [u8; 3],
    value: [u8; 16],
}

extern "C" {
    fn sdp_connect(src: *const [u8; 6], dst: *const [u8; 6], flags: u32) -> *mut c_void;
    fn sdp_close(session: *mut c_void) -> c_int;
    fn sdp_uuid128_create(uuid: *mut Uuid, data: *const c_void) -> *mut Uuid;
    fn sdp_list_append(list: *mut SdpList, data: *mut c_void) -> *mut SdpList;
    fn sdp_list_free(list: *mut SdpList, f: *const c_void);
    fn sdp_service_search_attr_req(
        session: *mut c_void,
        search: *mut SdpList,
        reqtype: c_int,
        attrid: *mut SdpList,
        rsp: *mut *mut SdpList,
    ) -> c_int;
    fn sdp_get_access_protos(rec: *const c_void, protos: *mut *mut SdpList) -> c_int;
    fn sdp_get_proto_port(list: *const SdpList, proto: c_int) -> c_int;
    fn sdp_record_free(rec: *mut c_void);
}

fn channel_for(address: &str, uuid: &[u8; 16]) -> u8 {
    let Some(target) = parse_bdaddr(address) else { return 0 };
    let any = [0u8; 6];
    let mut port = 0u8;
    // SAFETY: straightforward use of the libbluetooth SDP API; every list and
    // record allocated below is freed before returning.
    unsafe {
        let session = sdp_connect(&any, &target, SDP_RETRY_IF_BUSY);
        if session.is_null() {
            return 0;
        }
        let mut svc = Uuid { kind: 0, _pad: [0; 3], value: [0; 16] };
        sdp_uuid128_create(&mut svc, uuid.as_ptr().cast());
        let mut range: u32 = 0x0000_ffff;
        let search = sdp_list_append(ptr::null_mut(), (&mut svc as *mut Uuid).cast());
        let attrs = sdp_list_append(ptr::null_mut(), (&mut range as *mut u32).cast());
        let mut rsp: *mut SdpList = ptr::null_mut();
        if sdp_service_search_attr_req(session, search, SDP_ATTR_REQ_RANGE, attrs, &mut rsp) == 0 {
            let mut r = rsp;
            while !r.is_null() {
                let rec = (*r).data;
                let mut protos: *mut SdpList = ptr::null_mut();
                if sdp_get_access_protos(rec, &mut protos) == 0 {
                    let p = sdp_get_proto_port(protos, RFCOMM_UUID);
                    if p > 0 && p < 256 {
                        port = p as u8;
                    }
                    sdp_list_free(protos, ptr::null());
                }
                sdp_record_free(rec);
                r = (*r).next;
            }
        }
        sdp_list_free(rsp, ptr::null());
        sdp_list_free(search, ptr::null());
        sdp_list_free(attrs, ptr::null());
        sdp_close(session);
    }
    port
}

/// Tries the V1 service UUID first, then the newer V2 one.
pub fn find_sony_channel(address: &str) -> Option<(ProtocolKind, u8)> {
    let c = channel_for(address, &SERVICE_UUID_V1);
    if c != 0 {
        return Some((ProtocolKind::V1, c));
    }
    let c = channel_for(address, &SERVICE_UUID_V2);
    (c != 0).then_some((ProtocolKind::V2, c))
}
