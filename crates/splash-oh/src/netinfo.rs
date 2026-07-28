//! What the network actually is, from `libnet_connection`.
//!
//! `navigator.onLine` in a page is a boolean that mostly means "the browser has
//! not noticed a failure yet". This is the real thing: which bearer carries the
//! default route, whether the system considers the link validated (as opposed
//! to merely associated), whether it is metered, and whether a proxy is in
//! front of it.
//!
//! The proxy field earns its place. A stale global proxy pointing at a
//! `127.0.0.1` port nothing was listening on is exactly what made a phone in
//! this project look like it had no internet while the Wi-Fi showed connected —
//! a failure that is invisible from inside a page and obvious here.

use crate::bridge::json_str;
use std::os::raw::c_char;

const MAX_STR: usize = 256;
const MAX_CAP: usize = 32;
const MAX_BEARER: usize = 32;
const MAX_EXCLUSION: usize = 256;

#[repr(C)]
struct NetHandle {
    net_id: i32,
}

#[repr(C)]
struct NetCapabilities {
    link_up_bandwidth_kbps: u32,
    link_down_bandwidth_kbps: u32,
    net_caps: [i32; MAX_CAP],
    net_caps_size: i32,
    bearer_types: [i32; MAX_BEARER],
    bearer_types_size: i32,
}

#[repr(C)]
struct HttpProxy {
    host: [c_char; MAX_STR],
    exclusion_list: [[c_char; MAX_STR]; MAX_EXCLUSION],
    exclusion_list_size: i32,
    port: u16,
}

extern "C" {
    fn OH_NetConn_HasDefaultNet(has: *mut i32) -> i32;
    fn OH_NetConn_GetDefaultNet(handle: *mut NetHandle) -> i32;
    fn OH_NetConn_IsDefaultNetMetered(metered: *mut i32) -> i32;
    fn OH_NetConn_GetNetCapabilities(handle: *mut NetHandle, caps: *mut NetCapabilities) -> i32;
    fn OH_NetConn_GetDefaultHttpProxy(proxy: *mut HttpProxy) -> i32;
}

fn bearer_name(b: i32) -> &'static str {
    match b {
        0 => "cellular",
        1 => "wifi",
        2 => "bluetooth",
        3 => "ethernet",
        4 => "vpn",
        _ => "unknown",
    }
}

fn cap_name(c: i32) -> &'static str {
    match c {
        0 => "mms",
        11 => "not-metered",
        12 => "internet",
        15 => "not-vpn",
        16 => "validated",
        17 => "portal",
        31 => "checking-connectivity",
        _ => "other",
    }
}

fn c_str(buf: &[c_char]) -> String {
    let bytes: Vec<u8> = buf
        .iter()
        .map(|&c| c as u8)
        .take_while(|&c| c != 0)
        .collect();
    String::from_utf8_lossy(&bytes).into_owned()
}

/// The default route and what carries it. JSON object.
pub fn info() -> String {
    let mut has: i32 = 0;
    let has_net = unsafe { OH_NetConn_HasDefaultNet(&mut has) } == 0 && has != 0;

    let mut metered: i32 = 0;
    let metered_ok = unsafe { OH_NetConn_IsDefaultNetMetered(&mut metered) } == 0;

    let mut bearers: Vec<&str> = Vec::new();
    let mut caps: Vec<&str> = Vec::new();
    let mut net_id = -1;
    let mut down = 0u32;
    let mut up = 0u32;

    if has_net {
        let mut handle = NetHandle { net_id: 0 };
        if unsafe { OH_NetConn_GetDefaultNet(&mut handle) } == 0 {
            net_id = handle.net_id;
            // Zeroed rather than Default-derived: the struct is ~70 KB of
            // fixed arrays and only the *_size fields say how much is real.
            let mut c: NetCapabilities = unsafe { std::mem::zeroed() };
            if unsafe { OH_NetConn_GetNetCapabilities(&mut handle, &mut c) } == 0 {
                down = c.link_down_bandwidth_kbps;
                up = c.link_up_bandwidth_kbps;
                for i in 0..(c.bearer_types_size.max(0) as usize).min(MAX_BEARER) {
                    bearers.push(bearer_name(c.bearer_types[i]));
                }
                for i in 0..(c.net_caps_size.max(0) as usize).min(MAX_CAP) {
                    caps.push(cap_name(c.net_caps[i]));
                }
            }
        }
    }

    // Boxed: HttpProxy carries a 256 x 256 exclusion list, which is 64 KB and
    // has no business on the stack.
    let mut proxy: Box<HttpProxy> = unsafe { Box::new(std::mem::zeroed()) };
    let proxy_json = if unsafe { OH_NetConn_GetDefaultHttpProxy(&mut *proxy) } == 0 {
        let host = c_str(&proxy.host);
        if host.is_empty() {
            "null".to_string()
        } else {
            format!("{{\"host\":{},\"port\":{}}}", json_str(&host), proxy.port)
        }
    } else {
        "null".to_string()
    };

    let list = |v: &[&str]| v.iter().map(|s| json_str(s)).collect::<Vec<_>>().join(",");

    format!(
        "{{\"online\":{},\"netId\":{},\"metered\":{},\"bearers\":[{}],\"capabilities\":[{}],\
         \"downKbps\":{},\"upKbps\":{},\"proxy\":{}}}",
        has_net,
        net_id,
        if metered_ok {
            (metered != 0).to_string()
        } else {
            "null".into()
        },
        list(&bearers),
        list(&caps),
        down,
        up,
        proxy_json,
    )
}
