//! Radio, Wi-Fi and hashing — three more NDK kits.
//!
//! Grouped because each is one or two calls, and a module per extern block
//! would be more files than facts.
//!
//! | | library | needs |
//! |---|---|---|
//! | cellular network state | `libtelephony_radio` | `GET_NETWORK_INFO` |
//! | Wi-Fi enabled | `libwifi_ndk` | `GET_WIFI_INFO` |
//! | SHA-256 | `libohcrypto` | nothing |
//!
//! The MAC address getter in the Wi-Fi kit is deliberately not exposed. It is a
//! stable hardware identifier — the sort of thing that is a tracking primitive
//! rather than a feature — and nothing here needs it. `net.info` already says
//! whether Wi-Fi carries the default route, which is the part a card can use.

use crate::bridge::json_str;
use std::ffi::{c_void, CStr, CString};
use std::os::raw::c_char;

const MAX_OPERATOR: usize = 64;
const MAX_PLMN: usize = 6;

#[repr(C)]
struct NetworkState {
    long_operator: [c_char; MAX_OPERATOR],
    short_operator: [c_char; MAX_OPERATOR],
    plmn_numeric: [c_char; MAX_PLMN],
    is_roaming: bool,
    reg_state: i32,
    cfg_tech: i32,
    nsa_state: i32,
    is_ca_active: bool,
    is_emergency: bool,
}

#[repr(C)]
struct DataBlob {
    data: *mut u8,
    len: usize,
}

extern "C" {
    fn OH_Telephony_GetNetworkState(state: *mut NetworkState) -> i32;
    fn OH_Wifi_IsWifiEnabled(enabled: *mut bool) -> i32;

    fn OH_CryptoDigest_Create(algo: *const c_char, ctx: *mut *mut c_void) -> i32;
    fn OH_CryptoDigest_Update(ctx: *mut c_void, input: *mut DataBlob) -> i32;
    fn OH_CryptoDigest_Final(ctx: *mut c_void, out: *mut DataBlob) -> i32;
    fn OH_DigestCrypto_Destroy(ctx: *mut c_void);
}

fn c_str(buf: &[c_char]) -> String {
    let bytes: Vec<u8> = buf
        .iter()
        .map(|&c| c as u8)
        .take_while(|&c| c != 0)
        .collect();
    String::from_utf8_lossy(&bytes).into_owned()
}

fn tech_name(t: i32) -> &'static str {
    match t {
        1 => "GSM",
        2 => "1xRTT",
        3 => "WCDMA",
        4 => "HSPA",
        5 => "HSPA+",
        6 => "TD-SCDMA",
        7 => "EVDO",
        8 => "eHRPD",
        9 => "LTE",
        10 => "LTE-CA",
        11 => "IWLAN",
        12 => "NR",
        _ => "unknown",
    }
}

fn reg_name(r: i32) -> &'static str {
    match r {
        0 => "no-service",
        1 => "in-service",
        2 => "emergency-only",
        3 => "power-off",
        _ => "unknown",
    }
}

/// Cellular network state. JSON object, or an error.
pub fn cellular() -> Result<String, String> {
    // Zeroed rather than field-by-field: it is mostly fixed char arrays, and
    // the API fills whichever of them it has.
    let mut st: NetworkState = unsafe { std::mem::zeroed() };
    let rc = unsafe { OH_Telephony_GetNetworkState(&mut st) };
    if rc != 0 {
        return Err(match rc {
            201 => "permission denied (ohos.permission.GET_NETWORK_INFO)".into(),
            801 => "no cellular radio on this device".into(),
            _ => format!("radio state failed ({rc})"),
        });
    }
    Ok(format!(
        "{{\"operator\":{},\"operatorShort\":{},\"plmn\":{},\"roaming\":{},\
         \"registration\":{},\"technology\":{},\"carrierAggregation\":{},\"emergencyOnly\":{}}}",
        json_str(&c_str(&st.long_operator)),
        json_str(&c_str(&st.short_operator)),
        json_str(&c_str(&st.plmn_numeric)),
        st.is_roaming,
        json_str(reg_name(st.reg_state)),
        json_str(tech_name(st.cfg_tech)),
        st.is_ca_active,
        st.is_emergency,
    ))
}

/// Whether the Wi-Fi radio is switched on. Distinct from whether it carries the
/// default route, which is `net.info`'s job.
pub fn wifi() -> Result<String, String> {
    let mut on = false;
    let rc = unsafe { OH_Wifi_IsWifiEnabled(&mut on) };
    if rc != 0 {
        return Err(match rc {
            201 => "permission denied (ohos.permission.GET_WIFI_INFO)".into(),
            801 => "no Wi-Fi radio on this device".into(),
            _ => format!("wifi state failed ({rc})"),
        });
    }
    Ok(format!("{{\"enabled\":{on}}}"))
}

/// SHA-256 of a byte string, hex. Uses the system crypto rather than a
/// hand-rolled implementation — the point of a hash is that everyone computes
/// the same one, and the platform's is the one everything else on the device
/// agrees with.
pub fn sha256(input: &[u8]) -> Result<String, String> {
    let algo = CString::new("SHA256").map_err(|_| "bad algorithm name")?;
    let mut ctx: *mut c_void = std::ptr::null_mut();
    if unsafe { OH_CryptoDigest_Create(algo.as_ptr(), &mut ctx) } != 0 || ctx.is_null() {
        return Err("could not create a SHA-256 context".into());
    }

    let result = (|| {
        // The blob points at the caller's bytes; the kit does not take them.
        let mut inb = DataBlob {
            data: input.as_ptr() as *mut u8,
            len: input.len(),
        };
        if unsafe { OH_CryptoDigest_Update(ctx, &mut inb) } != 0 {
            return Err("digest update failed".to_string());
        }
        let mut out = DataBlob {
            data: std::ptr::null_mut(),
            len: 0,
        };
        if unsafe { OH_CryptoDigest_Final(ctx, &mut out) } != 0 || out.data.is_null() {
            return Err("digest final failed".to_string());
        }
        let bytes = unsafe { std::slice::from_raw_parts(out.data, out.len) };
        let hex: String = bytes.iter().map(|b| format!("{b:02x}")).collect();
        Ok(format!(
            "{{\"algorithm\":\"SHA-256\",\"hex\":{},\"inputBytes\":{}}}",
            json_str(&hex),
            input.len()
        ))
    })();

    unsafe { OH_DigestCrypto_Destroy(ctx) };
    result
}

/// SHA-256 of a file, so a page can check what it was handed without the bytes
/// crossing the bridge. Streamed in chunks: `fs.read` caps at 1 MB precisely
/// because moving a file through the reply queue is expensive, and hashing one
/// should not reintroduce that cost.
pub fn sha256_file(path: &str) -> Result<String, String> {
    use std::io::Read;
    let mut f = std::fs::File::open(path).map_err(|e| e.to_string())?;

    let algo = CString::new("SHA256").map_err(|_| "bad algorithm name")?;
    let mut ctx: *mut c_void = std::ptr::null_mut();
    if unsafe { OH_CryptoDigest_Create(algo.as_ptr(), &mut ctx) } != 0 || ctx.is_null() {
        return Err("could not create a SHA-256 context".into());
    }

    let result = (|| {
        let mut buf = vec![0u8; 64 * 1024];
        let mut total: u64 = 0;
        loop {
            let n = f.read(&mut buf).map_err(|e| e.to_string())?;
            if n == 0 {
                break;
            }
            total += n as u64;
            let mut inb = DataBlob {
                data: buf.as_mut_ptr(),
                len: n,
            };
            if unsafe { OH_CryptoDigest_Update(ctx, &mut inb) } != 0 {
                return Err("digest update failed".to_string());
            }
        }
        let mut out = DataBlob {
            data: std::ptr::null_mut(),
            len: 0,
        };
        if unsafe { OH_CryptoDigest_Final(ctx, &mut out) } != 0 || out.data.is_null() {
            return Err("digest final failed".to_string());
        }
        let bytes = unsafe { std::slice::from_raw_parts(out.data, out.len) };
        let hex: String = bytes.iter().map(|b| format!("{b:02x}")).collect();
        Ok(format!(
            "{{\"algorithm\":\"SHA-256\",\"hex\":{},\"path\":{},\"bytes\":{}}}",
            json_str(&hex),
            json_str(path),
            total
        ))
    })();

    unsafe { OH_DigestCrypto_Destroy(ctx) };
    result
}

/// Unused today; kept because the digest kit's algorithm name is queryable and
/// the next hash added will want to report it rather than hardcode a string.
#[allow(dead_code)]
fn algo_of(ctx: *mut c_void) -> String {
    extern "C" {
        fn OH_CryptoDigest_GetAlgoName(ctx: *mut c_void) -> *const c_char;
    }
    let p = unsafe { OH_CryptoDigest_GetAlgoName(ctx) };
    if p.is_null() {
        String::new()
    } else {
        unsafe { CStr::from_ptr(p) }.to_string_lossy().into_owned()
    }
}
