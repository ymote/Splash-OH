//! Keystore, relational database, and Bluetooth state.
//!
//! Three kits grouped because each is small on its own, and all three are
//! `dlopen`ed for the reason #26 established: an unresolved `DT_NEEDED` symbol
//! is fatal to the whole library, so an optional capability must not be able to
//! take the rest down with it.
//!
//! | | library | what |
//! |---|---|---|
//! | `keystore.*` | `libhuks_ndk.z` | keys generated inside the secure store |
//! | `db.*` | `libnative_rdb_ndk.z` | real SQLite, not the hand-rolled `prefs` |
//! | `bluetooth.state` | `libbluetooth_ndk` | adapter switch state |
//!
//! # What HUKS is for, as distinct from `crypto.sha256`
//!
//! `crypto.sha256` computes over data the caller already holds. HUKS is the
//! opposite: the key material is generated **inside** the keystore and there is
//! no API that hands it back. `OH_Huks_ExportPublicKeyItem` exports the public
//! half of an asymmetric pair and nothing exports the private half — that is
//! the entire point. A page can ask for a key to exist and can never learn it.
//!
//! # Bluetooth is one function
//!
//! `OH_Bluetooth_GetBluetoothSwitchState` is the whole native surface. Scanning,
//! pairing and GATT are ArkTS-only, so a BLE scan would need the Rust → ArkTS
//! channel rather than a few more `dlsym` calls. Not done here.

use crate::bridge::json_str;
use std::ffi::{c_void, CString};
use std::os::raw::c_char;

extern "C" {
    fn dlopen(file: *const c_char, mode: i32) -> *mut c_void;
    fn dlsym(handle: *mut c_void, name: *const c_char) -> *mut c_void;
}

fn open_lib(name: &str) -> Option<*mut c_void> {
    let c = CString::new(name).ok()?;
    let h = unsafe { dlopen(c.as_ptr(), 2) };
    if h.is_null() {
        crate::log(&format!("secure: {name} did not load"));
        return None;
    }
    Some(h)
}

fn sym<T>(h: *mut c_void, name: &str) -> Option<T> {
    let c = CString::new(name).ok()?;
    let p = unsafe { dlsym(h, c.as_ptr()) };
    if p.is_null() {
        crate::log(&format!("secure: symbol {name} not found"));
        return None;
    }
    Some(unsafe { std::mem::transmute_copy(&p) })
}

// --- HUKS -------------------------------------------------------------------

#[repr(C)]
struct HuksBlob {
    size: u32,
    data: *mut u8,
}

#[repr(C)]
struct HuksResult {
    error_code: i32,
    error_msg: *const c_char,
    data: *mut c_void,
}

/// `OH_Huks_Param` is a tag plus a union. Only the u32 arm is used here, and
/// the union is sized by its largest member (a blob: u32 + pointer = 16 bytes
/// with alignment), so the struct is laid out by hand to match.
#[repr(C)]
struct HuksParam {
    tag: u32,
    _pad: u32,
    value: u64,
    _rest: u64,
}

struct HuksApi {
    sdk_version: unsafe extern "C" fn(*mut HuksBlob) -> HuksResult,
    init_param_set: unsafe extern "C" fn(*mut *mut c_void) -> HuksResult,
    add_params: unsafe extern "C" fn(*mut c_void, *const HuksParam, u32) -> HuksResult,
    build_param_set: unsafe extern "C" fn(*mut *mut c_void) -> HuksResult,
    free_param_set: unsafe extern "C" fn(*mut *mut c_void) -> HuksResult,
    generate_key: unsafe extern "C" fn(*const HuksBlob, *const c_void, *mut c_void) -> HuksResult,
    key_exists: unsafe extern "C" fn(*const HuksBlob, *const c_void) -> HuksResult,
    delete_key: unsafe extern "C" fn(*const HuksBlob, *const c_void) -> HuksResult,
}

static HUKS: std::sync::OnceLock<Option<HuksApi>> = std::sync::OnceLock::new();

fn huks() -> Option<&'static HuksApi> {
    HUKS.get_or_init(|| {
        let h = open_lib("libhuks_ndk.z.so")?;
        Some(HuksApi {
            sdk_version: sym(h, "OH_Huks_GetSdkVersion")?,
            init_param_set: sym(h, "OH_Huks_InitParamSet")?,
            add_params: sym(h, "OH_Huks_AddParams")?,
            build_param_set: sym(h, "OH_Huks_BuildParamSet")?,
            free_param_set: sym(h, "OH_Huks_FreeParamSet")?,
            generate_key: sym(h, "OH_Huks_GenerateKeyItem")?,
            key_exists: sym(h, "OH_Huks_IsKeyItemExist")?,
            delete_key: sym(h, "OH_Huks_DeleteKeyItem")?,
        })
    })
    .as_ref()
}

// Tags from native_huks_type.h. The parameter's *type* is encoded in the high
// bits, and getting that base wrong is not a subtle failure -- it made every
// generate call return 401 (invalid parameter), because HUKS read a UINT tag
// as an INT one and rejected the set.
//
//   OH_HUKS_TAG_TYPE_INT  = 1 << 28
//   OH_HUKS_TAG_TYPE_UINT = 2 << 28   <- these four are UINT
const HUKS_TAG_TYPE_UINT: u32 = 2 << 28;
const HUKS_TAG_ALGORITHM: u32 = HUKS_TAG_TYPE_UINT | 1;
const HUKS_TAG_PURPOSE: u32 = HUKS_TAG_TYPE_UINT | 2;
const HUKS_TAG_KEY_SIZE: u32 = HUKS_TAG_TYPE_UINT | 3;
const HUKS_TAG_DIGEST: u32 = HUKS_TAG_TYPE_UINT | 4;
// Read from the header, not guessed. Two of these were wrong on the first
// attempt -- ECC as 5 and SHA256 as 4 -- and HUKS answered 12000003
// (INVALID_CRYPTO_ALG_ARGUMENT) rather than naming the offending parameter.
// An enum in a C header is cheap to check and expensive to assume.
const HUKS_ALG_ECC: u32 = 2;
const HUKS_PURPOSE_SIGN: u32 = 4;
const HUKS_PURPOSE_VERIFY: u32 = 8;
const HUKS_ECC_KEY_SIZE_256: u32 = 256;
const HUKS_DIGEST_SHA256: u32 = 12;

const NO_HUKS: &str = "keystore unavailable on this device";

fn blob(s: &str) -> HuksBlob {
    HuksBlob {
        size: s.len() as u32,
        data: s.as_ptr() as *mut u8,
    }
}

/// Generate an ECC P-256 signing key that lives inside the keystore, confirm it
/// exists, and delete it.
///
/// The round trip is the demonstration. There is no call anywhere in this API
/// that returns the private key, so what a page gets from this is the knowledge
/// that a key was created and destroyed — never the key.
pub fn keystore_roundtrip(alias: &str) -> Result<String, String> {
    let api = huks().ok_or(NO_HUKS)?;

    let mut ver_buf = vec![0u8; 64];
    let mut ver = HuksBlob {
        size: ver_buf.len() as u32,
        data: ver_buf.as_mut_ptr(),
    };
    let r = unsafe { (api.sdk_version)(&mut ver) };
    let version = if r.error_code == 0 {
        String::from_utf8_lossy(&ver_buf[..ver.size as usize])
            .trim_end_matches('\0')
            .to_string()
    } else {
        String::new()
    };

    let a = blob(alias);

    // An empty param set for the existence and delete calls; generation needs
    // the real one.
    let mut gen_set: *mut c_void = std::ptr::null_mut();
    let r = unsafe { (api.init_param_set)(&mut gen_set) };
    if r.error_code != 0 || gen_set.is_null() {
        return Err(format!("param set: {}", r.error_code));
    }

    let out = (|| -> Result<String, String> {
        let params = [
            HuksParam {
                tag: HUKS_TAG_ALGORITHM,
                _pad: 0,
                value: HUKS_ALG_ECC as u64,
                _rest: 0,
            },
            HuksParam {
                tag: HUKS_TAG_PURPOSE,
                _pad: 0,
                value: (HUKS_PURPOSE_SIGN | HUKS_PURPOSE_VERIFY) as u64,
                _rest: 0,
            },
            HuksParam {
                tag: HUKS_TAG_KEY_SIZE,
                _pad: 0,
                value: HUKS_ECC_KEY_SIZE_256 as u64,
                _rest: 0,
            },
            HuksParam {
                tag: HUKS_TAG_DIGEST,
                _pad: 0,
                value: HUKS_DIGEST_SHA256 as u64,
                _rest: 0,
            },
        ];
        let r = unsafe { (api.add_params)(gen_set, params.as_ptr(), params.len() as u32) };
        if r.error_code != 0 {
            return Err(format!("add params: {}", r.error_code));
        }
        let r = unsafe { (api.build_param_set)(&mut gen_set) };
        if r.error_code != 0 {
            return Err(format!("build params: {}", r.error_code));
        }

        // Start clean: a leftover alias from a previous run would make the
        // "generated" result meaningless.
        unsafe { (api.delete_key)(&a, gen_set) };

        let r = unsafe { (api.generate_key)(&a, gen_set, std::ptr::null_mut()) };
        if r.error_code != 0 {
            return Err(format!("generate: {}", r.error_code));
        }
        let existed = unsafe { (api.key_exists)(&a, gen_set) }.error_code == 0;
        let deleted = unsafe { (api.delete_key)(&a, gen_set) }.error_code == 0;
        let gone = unsafe { (api.key_exists)(&a, gen_set) }.error_code != 0;

        Ok(format!(
            "{{\"sdkVersion\":{},\"alias\":{},\"algorithm\":\"ECC P-256\",\
             \"generated\":true,\"existed\":{},\"deleted\":{},\"goneAfterDelete\":{},\
             \"privateKeyExportable\":false}}",
            json_str(&version),
            json_str(alias),
            existed,
            deleted,
            gone
        ))
    })();

    unsafe { (api.free_param_set)(&mut gen_set) };
    out
}

// --- RDB --------------------------------------------------------------------

/// `#[repr(C, packed)]`, because the header declares this inside
/// `#pragma pack(1)`.
///
/// This is not cosmetic. With natural alignment the struct is 8 bytes larger,
/// every field after `selfSize` sits at the wrong offset, and `OH_Rdb_GetOrOpen`
/// returned a null store with error code 0 -- a failure that reports nothing
/// because the library never got a config it could read.
#[repr(C, packed)]
struct RdbConfig {
    self_size: i32,
    data_base_dir: *const c_char,
    store_name: *const c_char,
    bundle_name: *const c_char,
    module_name: *const c_char,
    is_encrypt: bool,
    security_level: i32,
    area: i32,
}

/// `OH_Cursor` is a vtable struct; only the four entries needed here are
/// declared, and they are at the documented offsets.
#[repr(C)]
struct Cursor {
    id: i64,
    get_column_count: unsafe extern "C" fn(*mut Cursor, *mut i32) -> i32,
    get_column_type: unsafe extern "C" fn(*mut Cursor, i32, *mut i32) -> i32,
    get_column_index: unsafe extern "C" fn(*mut Cursor, *const c_char, *mut i32) -> i32,
    get_column_name: unsafe extern "C" fn(*mut Cursor, i32, *mut c_char, i32) -> i32,
    get_row_count: unsafe extern "C" fn(*mut Cursor, *mut i32) -> i32,
    go_to_next_row: unsafe extern "C" fn(*mut Cursor) -> i32,
    get_size: unsafe extern "C" fn(*mut Cursor, i32, *mut usize) -> i32,
    get_text: unsafe extern "C" fn(*mut Cursor, i32, *mut c_char, i32) -> i32,
    get_int64: unsafe extern "C" fn(*mut Cursor, i32, *mut i64) -> i32,
    get_real: unsafe extern "C" fn(*mut Cursor, i32, *mut f64) -> i32,
    get_blob: unsafe extern "C" fn(*mut Cursor, i32, *mut u8, i32) -> i32,
    is_null: unsafe extern "C" fn(*mut Cursor, i32, *mut bool) -> i32,
    destroy: unsafe extern "C" fn(*mut Cursor) -> i32,
}

struct RdbApi {
    get_or_open: unsafe extern "C" fn(*const RdbConfig, *mut i32) -> *mut c_void,
    close_store: unsafe extern "C" fn(*mut c_void) -> i32,
    execute: unsafe extern "C" fn(*mut c_void, *const c_char) -> i32,
    execute_query: unsafe extern "C" fn(*mut c_void, *const c_char) -> *mut Cursor,
}

static RDB: std::sync::OnceLock<Option<RdbApi>> = std::sync::OnceLock::new();

fn rdb() -> Option<&'static RdbApi> {
    RDB.get_or_init(|| {
        let h = open_lib("libnative_rdb_ndk.z.so")?;
        Some(RdbApi {
            get_or_open: sym(h, "OH_Rdb_GetOrOpen")?,
            close_store: sym(h, "OH_Rdb_CloseStore")?,
            execute: sym(h, "OH_Rdb_Execute")?,
            execute_query: sym(h, "OH_Rdb_ExecuteQuery")?,
        })
    })
    .as_ref()
}

const NO_RDB: &str = "relational store unavailable on this device";

/// Create a table, insert rows, and read them back with SQL.
///
/// This is what `prefs` is a stand-in for. `prefs` rewrites one JSON file whole,
/// which is correct at a few dozen keys and wrong the moment anything wants a
/// query. Here the database does the work, and the row count comes back from
/// `SELECT count(*)` rather than from anything this code counted itself.
pub fn db_roundtrip() -> Result<String, String> {
    let api = rdb().ok_or(NO_RDB)?;

    let dir = CString::new("/data/storage/el2/base/files").map_err(|_| "bad dir")?;
    let store = CString::new("splash.db").map_err(|_| "bad name")?;
    let bundle = CString::new("com.example.myapplication").map_err(|_| "bad bundle")?;
    let module = CString::new("entry").map_err(|_| "bad module")?;

    let config = RdbConfig {
        self_size: std::mem::size_of::<RdbConfig>() as i32,
        data_base_dir: dir.as_ptr(),
        store_name: store.as_ptr(),
        bundle_name: bundle.as_ptr(),
        module_name: module.as_ptr(),
        is_encrypt: false,
        // S1: the lowest sensitivity. Nothing here is a secret; a key that
        // mattered would live in HUKS above, not in a row.
        security_level: 1,
        area: 1,
    };

    let mut rc: i32 = 0;
    let s = unsafe { (api.get_or_open)(&config, &mut rc) };
    if s.is_null() {
        return Err(format!("open failed ({rc})"));
    }

    let out = (|| -> Result<String, String> {
        let exec = |sql: &str| -> Result<(), String> {
            let c = CString::new(sql).map_err(|_| "bad sql")?;
            let rc = unsafe { (api.execute)(s, c.as_ptr()) };
            if rc != 0 {
                return Err(format!("{sql} -> {rc}"));
            }
            Ok(())
        };

        exec("DROP TABLE IF EXISTS readings")?;
        exec("CREATE TABLE readings (id INTEGER PRIMARY KEY, kind TEXT, value REAL)")?;
        exec("INSERT INTO readings (kind, value) VALUES ('lux', 41.3)")?;
        exec("INSERT INTO readings (kind, value) VALUES ('lux', 65.5)")?;
        exec("INSERT INTO readings (kind, value) VALUES ('accel_z', 9.81)")?;

        // The aggregate is the point: this is arithmetic the database did, not
        // something the caller computed and is reporting back to itself.
        let q =
            CString::new("SELECT count(*), round(avg(value), 2) FROM readings WHERE kind='lux'")
                .map_err(|_| "bad sql")?;
        let cur = unsafe { (api.execute_query)(s, q.as_ptr()) };
        if cur.is_null() {
            return Err("query returned no cursor".into());
        }
        let (count, avg) = unsafe {
            ((*cur).go_to_next_row)(cur);
            let mut n: i64 = 0;
            let mut a: f64 = 0.0;
            ((*cur).get_int64)(cur, 0, &mut n);
            ((*cur).get_real)(cur, 1, &mut a);
            ((*cur).destroy)(cur);
            (n, a)
        };

        Ok(format!(
            "{{\"engine\":\"SQLite via OH_Rdb\",\"luxRows\":{count},\"luxAverage\":{avg},\
             \"path\":\"/data/storage/el2/base/files/splash.db\"}}"
        ))
    })();

    unsafe { (api.close_store)(s) };
    out
}

// --- Bluetooth --------------------------------------------------------------

static BT: std::sync::OnceLock<Option<unsafe extern "C" fn(*mut i32) -> i32>> =
    std::sync::OnceLock::new();

/// Adapter switch state. The entire native Bluetooth surface — scanning and
/// GATT are ArkTS-only.
pub fn bluetooth_state() -> Result<String, String> {
    let f = BT
        .get_or_init(|| {
            let h = open_lib("libbluetooth_ndk.so")?;
            sym(h, "OH_Bluetooth_GetBluetoothSwitchState")
        })
        .ok_or("bluetooth kit unavailable on this device")?;

    let mut state: i32 = -1;
    let rc = unsafe { f(&mut state) };
    if rc != 0 {
        return Err(match rc {
            201 => "permission denied (ohos.permission.ACCESS_BLUETOOTH)".into(),
            801 => "bluetooth not supported on this device".into(),
            _ => format!("bluetooth state failed ({rc})"),
        });
    }
    let name = match state {
        0 => "off",
        1 => "turning on",
        2 => "on",
        3 => "turning off",
        _ => "unknown",
    };
    Ok(format!(
        "{{\"state\":{},\"name\":{},\"note\":\"scanning and GATT are ArkTS-only\"}}",
        state,
        json_str(name)
    ))
}
