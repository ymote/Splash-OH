//! Image decoding and encoding, and camera enumeration.
//!
//! These are the first NDK kits here that are *pipelines* rather than getters:
//! a source is created, decoded into a pixel map under options, and a packer
//! turns that back into bytes. Every stage allocates something that must be
//! released on the way out, including the error paths — hence the nested
//! closures, which is the Rust shape for "run this, then unwind whatever
//! happened".
//!
//! # Why a thumbnail is the useful operation
//!
//! `fs.pick` can hand a page the path of a photo, and `fs.read` can hand it the
//! bytes — but a phone photo is several megabytes, and everything on this
//! bridge crosses as one JSON string that is then evaluated into the page. A
//! full-size image is the one payload shape this channel genuinely cannot
//! carry.
//!
//! Decoding at a reduced size and re-encoding as JPEG fixes that at the source:
//! `OH_DecodingOptions_SetDesiredSize` means the decoder never materialises the
//! full bitmap, so a 12 MP photo becomes a ~30 KB data URI without 48 MB ever
//! existing. That turns "the page can be told a file exists" into "the page can
//! show it".

use crate::bridge::json_str;
use std::ffi::{c_void, CString};
use std::os::raw::c_char;
use std::sync::Mutex;

#[repr(C)]
struct ImageString {
    data: *mut c_char,
    size: usize,
}

#[repr(C)]
struct ImageSize {
    width: u32,
    height: u32,
}

/// The image kit, resolved at runtime.
///
/// These are `dlopen`ed rather than linked, and that is not a style choice.
/// Linking them added a `DT_NEEDED` the loader on this device could not
/// satisfy, and an unsatisfied `DT_NEEDED` is fatal to the *whole* library —
/// the app died during launch, before any of this code ran, taking the sensors,
/// the network and the filesystem tools down with it. A capability that might
/// not exist has no business being able to do that.
///
/// Resolved once; a kit that is absent leaves `None` and its tools report
/// "unavailable" while everything else keeps working.
struct ImageApi {
    source_from_uri: unsafe extern "C" fn(*mut c_char, usize, *mut *mut c_void) -> i32,
    source_release: unsafe extern "C" fn(*mut c_void) -> i32,
    source_get_info: unsafe extern "C" fn(*mut c_void, i32, *mut c_void) -> i32,
    source_create_pixelmap: unsafe extern "C" fn(*mut c_void, *mut c_void, *mut *mut c_void) -> i32,
    info_create: unsafe extern "C" fn(*mut *mut c_void) -> i32,
    info_width: unsafe extern "C" fn(*mut c_void, *mut u32) -> i32,
    info_height: unsafe extern "C" fn(*mut c_void, *mut u32) -> i32,
    info_hdr: unsafe extern "C" fn(*mut c_void, *mut bool) -> i32,
    info_release: unsafe extern "C" fn(*mut c_void) -> i32,
    dec_create: unsafe extern "C" fn(*mut *mut c_void) -> i32,
    dec_set_size: unsafe extern "C" fn(*mut c_void, *mut ImageSize) -> i32,
    dec_release: unsafe extern "C" fn(*mut c_void) -> i32,
    pack_opts_create: unsafe extern "C" fn(*mut *mut c_void) -> i32,
    pack_opts_mime: unsafe extern "C" fn(*mut c_void, *mut ImageString) -> i32,
    pack_opts_quality: unsafe extern "C" fn(*mut c_void, u32) -> i32,
    pack_opts_release: unsafe extern "C" fn(*mut c_void) -> i32,
    packer_create: unsafe extern "C" fn(*mut *mut c_void) -> i32,
    packer_from_pixelmap:
        unsafe extern "C" fn(*mut c_void, *mut c_void, *mut c_void, *mut u8, *mut usize) -> i32,
    packer_release: unsafe extern "C" fn(*mut c_void) -> i32,
    pm_release: unsafe extern "C" fn(*mut c_void) -> i32,
    pm_info_create: unsafe extern "C" fn(*mut *mut c_void) -> i32,
    pm_info_width: unsafe extern "C" fn(*mut c_void, *mut u32) -> i32,
    pm_info_height: unsafe extern "C" fn(*mut c_void, *mut u32) -> i32,
    pm_info_release: unsafe extern "C" fn(*mut c_void) -> i32,
    pm_get_info: unsafe extern "C" fn(*mut c_void, *mut c_void) -> i32,
}

struct CameraApi {
    get_manager: unsafe extern "C" fn(*mut *mut c_void) -> i32,
    del_manager: unsafe extern "C" fn(*mut c_void) -> i32,
    supported: unsafe extern "C" fn(*mut c_void, *mut *mut CameraDevice, *mut u32) -> i32,
    del_supported: unsafe extern "C" fn(*mut c_void, *mut CameraDevice, u32) -> i32,
}

extern "C" {
    fn dlopen(file: *const c_char, mode: i32) -> *mut c_void;
    fn dlsym(handle: *mut c_void, name: *const c_char) -> *mut c_void;
}

const RTLD_NOW: i32 = 2;

/// Open the first of `names` that loads. The kits ship under more than one
/// soname across builds, and guessing wrong should mean "try the next" rather
/// than "no images".
fn open_any(names: &[&str]) -> Option<*mut c_void> {
    for n in names {
        let c = CString::new(*n).ok()?;
        let h = unsafe { dlopen(c.as_ptr(), RTLD_NOW) };
        if !h.is_null() {
            crate::log(&format!("image: loaded {n}"));
            return Some(h);
        }
    }
    crate::log(&format!("image: none of {names:?} could be loaded"));
    None
}

/// Resolve `name` from any of the open handles.
fn sym<T>(handles: &[*mut c_void], name: &str) -> Option<T> {
    let c = CString::new(name).ok()?;
    for h in handles {
        let p = unsafe { dlsym(*h, c.as_ptr()) };
        if !p.is_null() {
            return Some(unsafe { std::mem::transmute_copy(&p) });
        }
    }
    crate::log(&format!("image: symbol {name} not found"));
    None
}

static IMAGE_API: std::sync::OnceLock<Option<ImageApi>> = std::sync::OnceLock::new();
static CAMERA_API: std::sync::OnceLock<Option<CameraApi>> = std::sync::OnceLock::new();

fn image_api() -> Option<&'static ImageApi> {
    IMAGE_API
        .get_or_init(|| {
            let mut hs = Vec::new();
            hs.extend(open_any(&[
                "libimage_source_ndk.z.so",
                "libimage_source.so",
            ]));
            hs.extend(open_any(&[
                "libimage_packer_ndk.z.so",
                "libimage_packer.so",
            ]));
            hs.extend(open_any(&["libpixelmap.so", "libpixelmap_ndk.z.so"]));
            if hs.is_empty() {
                return None;
            }
            let api = (|| {
                Some(ImageApi {
                    source_from_uri: sym(&hs, "OH_ImageSourceNative_CreateFromUri")?,
                    source_release: sym(&hs, "OH_ImageSourceNative_Release")?,
                    source_get_info: sym(&hs, "OH_ImageSourceNative_GetImageInfo")?,
                    source_create_pixelmap: sym(&hs, "OH_ImageSourceNative_CreatePixelmap")?,
                    info_create: sym(&hs, "OH_ImageSourceInfo_Create")?,
                    info_width: sym(&hs, "OH_ImageSourceInfo_GetWidth")?,
                    info_height: sym(&hs, "OH_ImageSourceInfo_GetHeight")?,
                    info_hdr: sym(&hs, "OH_ImageSourceInfo_GetDynamicRange")?,
                    info_release: sym(&hs, "OH_ImageSourceInfo_Release")?,
                    dec_create: sym(&hs, "OH_DecodingOptions_Create")?,
                    dec_set_size: sym(&hs, "OH_DecodingOptions_SetDesiredSize")?,
                    dec_release: sym(&hs, "OH_DecodingOptions_Release")?,
                    pack_opts_create: sym(&hs, "OH_PackingOptions_Create")?,
                    pack_opts_mime: sym(&hs, "OH_PackingOptions_SetMimeType")?,
                    pack_opts_quality: sym(&hs, "OH_PackingOptions_SetQuality")?,
                    pack_opts_release: sym(&hs, "OH_PackingOptions_Release")?,
                    packer_create: sym(&hs, "OH_ImagePackerNative_Create")?,
                    packer_from_pixelmap: sym(&hs, "OH_ImagePackerNative_PackToDataFromPixelmap")?,
                    packer_release: sym(&hs, "OH_ImagePackerNative_Release")?,
                    pm_release: sym(&hs, "OH_PixelmapNative_Release")?,
                    pm_info_create: sym(&hs, "OH_PixelmapImageInfo_Create")?,
                    pm_info_width: sym(&hs, "OH_PixelmapImageInfo_GetWidth")?,
                    pm_info_height: sym(&hs, "OH_PixelmapImageInfo_GetHeight")?,
                    pm_info_release: sym(&hs, "OH_PixelmapImageInfo_Release")?,
                    pm_get_info: sym(&hs, "OH_PixelmapNative_GetImageInfo")?,
                })
            })();
            if api.is_none() {
                // Say which half of the kit this device actually has. The
                // libraries load; it is the symbols that are absent.
                probe_symbols();
            }
            api
        })
        .as_ref()
}

fn camera_api() -> Option<&'static CameraApi> {
    CAMERA_API
        .get_or_init(|| {
            let h = open_any(&["libohcamera.so"])?;
            let hs = [h];
            Some(CameraApi {
                get_manager: sym(&hs, "OH_Camera_GetCameraManager")?,
                del_manager: sym(&hs, "OH_Camera_DeleteCameraManager")?,
                supported: sym(&hs, "OH_CameraManager_GetSupportedCameras")?,
                del_supported: sym(&hs, "OH_CameraManager_DeleteSupportedCameras")?,
            })
        })
        .as_ref()
}

/// One-off: report which of the plausible entry points this device actually
/// exports. `CreateFromUri` was absent even though every library loaded, which
/// is precisely the failure mode that made linking them fatal.
fn probe_symbols() {
    let mut hs = Vec::new();
    hs.extend(open_any(&[
        "libimage_source_ndk.z.so",
        "libimage_source.so",
    ]));
    hs.extend(open_any(&[
        "libimage_packer_ndk.z.so",
        "libimage_packer.so",
    ]));
    hs.extend(open_any(&["libpixelmap.so", "libpixelmap_ndk.z.so"]));
    for name in [
        "OH_ImageSourceNative_CreateFromUri",
        "OH_ImageSourceNative_CreateFromFd",
        "OH_ImageSourceNative_CreateFromData",
        "OH_ImageSourceNative_CreatePixelmap",
        "OH_ImageSourceNative_GetImageInfo",
        "OH_ImageSourceNative_Release",
        "OH_ImageSourceInfo_Create",
        "OH_DecodingOptions_Create",
        "OH_PackingOptions_Create",
        "OH_ImagePackerNative_Create",
        "OH_ImagePackerNative_PackToDataFromPixelmap",
        "OH_PixelmapNative_CreatePixelmap",
        "OH_PixelmapNative_Release",
        "OH_PixelmapImageInfo_Create",
        "OH_ImageSource_Create",
        "OH_ImageSource_CreatePixelMap",
    ] {
        let c = CString::new(name).unwrap();
        let mut found = false;
        for h in &hs {
            if !unsafe { dlsym(*h, c.as_ptr()) }.is_null() {
                found = true;
                break;
            }
        }
        crate::log(&format!(
            "probe {name}: {}",
            if found { "YES" } else { "no" }
        ));
    }
}

/// Why the image tools report unavailable on this device, stated once.
///
/// Every image library loads. What is missing is the whole `*Native*` symbol
/// family the API-24 headers declare -- `OH_ImageSourceNative_CreateFromUri`
/// and its relatives are simply not exported by this HarmonyOS 6.1 runtime.
/// What *is* exported is the legacy `OH_ImageSource_Create` family, and that
/// one takes a `napi_env` and returns a `napi_value`: it can only be driven
/// from the JS thread, which is precisely the thread these tools exist not to
/// block. So the codecs are unreachable from this bridge here, and the SDK
/// headers are ahead of the device.
const NO_IMAGE: &str =
    "image codecs unavailable: this build exports only the napi-based legacy API";
const NO_CAMERA: &str = "camera kit unavailable on this device";

fn err_message(rc: i32) -> String {
    match rc {
        401 => "invalid parameter".into(),
        7_800_201 => "unsupported image format".into(),
        7_800_301 => "decode failed".into(),
        7_800_302 => "encode failed".into(),
        _ => format!("image error ({rc})"),
    }
}

/// A file URI the image kit accepts. It wants a `file://` URI, not a bare path.
fn as_uri(path: &str) -> String {
    if path.starts_with("file://") {
        path.to_string()
    } else {
        format!("file://{path}")
    }
}

/// Run `f` with an image source for `path`, releasing it afterwards whatever
/// happens.
fn with_source<T>(
    path: &str,
    f: impl FnOnce(*mut c_void) -> Result<T, String>,
) -> Result<T, String> {
    let api = image_api().ok_or(NO_IMAGE)?;
    let uri = CString::new(as_uri(path)).map_err(|_| "path contains a NUL")?;
    let bytes = uri.as_bytes();
    let mut src: *mut c_void = std::ptr::null_mut();
    let rc = unsafe { (api.source_from_uri)(bytes.as_ptr() as *mut c_char, bytes.len(), &mut src) };
    if rc != 0 || src.is_null() {
        return Err(format!("could not open image: {}", err_message(rc)));
    }
    let out = f(src);
    unsafe { (api.source_release)(src) };
    out
}

/// Header-only: what an image is, without decoding it.
///
/// Deliberately separate from `thumbnail`. Learning that a 12 MP HEIF was
/// picked costs a header read; producing a preview of it costs a decode, and a
/// page often only needs the first.
pub fn info(path: &str) -> Result<String, String> {
    with_source(path, |src| {
        let api = image_api().ok_or(NO_IMAGE)?;
        let mut info: *mut c_void = std::ptr::null_mut();
        if unsafe { (api.info_create)(&mut info) } != 0 || info.is_null() {
            return Err("could not allocate image info".into());
        }
        let out = (|| {
            let rc = unsafe { (api.source_get_info)(src, 0, info) };
            if rc != 0 {
                return Err(err_message(rc));
            }
            let mut w = 0u32;
            let mut h = 0u32;
            let mut hdr = false;
            unsafe {
                (api.info_width)(info, &mut w);
                (api.info_height)(info, &mut h);
                (api.info_hdr)(info, &mut hdr);
            }
            let bytes = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);
            Ok(format!(
                "{{\"path\":{},\"width\":{},\"height\":{},\"megapixels\":{:.1},\
                 \"hdr\":{},\"fileBytes\":{}}}",
                json_str(path),
                w,
                h,
                (w as f64 * h as f64) / 1_000_000.0,
                hdr,
                bytes
            ))
        })();
        unsafe { (api.info_release)(info) };
        out
    })
}

/// Decode at reduced size and re-encode as JPEG. Returns a `data:` URI the page
/// can put straight in an `<img src>`.
///
/// `max_edge` bounds the longer side. The aspect ratio is preserved here rather
/// than left to the decoder, because `SetDesiredSize` takes an exact size and
/// will happily distort an image if handed a square.
pub fn thumbnail(path: &str, max_edge: u32, quality: u32) -> Result<String, String> {
    let api = image_api().ok_or(NO_IMAGE)?;
    let max_edge = max_edge.clamp(16, 1024);
    let quality = quality.clamp(1, 100);

    with_source(path, |src| {
        let api = image_api().ok_or(NO_IMAGE)?;
        // Read the real dimensions first, so the requested size keeps the shape.
        let (sw, sh) = {
            let mut info: *mut c_void = std::ptr::null_mut();
            if unsafe { (api.info_create)(&mut info) } != 0 || info.is_null() {
                return Err("could not allocate image info".into());
            }
            let mut w = 0u32;
            let mut h = 0u32;
            let rc = unsafe { (api.source_get_info)(src, 0, info) };
            if rc == 0 {
                unsafe {
                    (api.info_width)(info, &mut w);
                    (api.info_height)(info, &mut h);
                }
            }
            unsafe { (api.info_release)(info) };
            if rc != 0 || w == 0 || h == 0 {
                return Err(format!("could not read image size: {}", err_message(rc)));
            }
            (w, h)
        };

        let scale = (max_edge as f64 / sw.max(sh) as f64).min(1.0);
        let mut want = ImageSize {
            width: ((sw as f64 * scale).round() as u32).max(1),
            height: ((sh as f64 * scale).round() as u32).max(1),
        };

        let mut opts: *mut c_void = std::ptr::null_mut();
        if unsafe { (api.dec_create)(&mut opts) } != 0 || opts.is_null() {
            return Err("could not allocate decoding options".into());
        }
        let decoded = (|| {
            unsafe { (api.dec_set_size)(opts, &mut want) };
            let mut pm: *mut c_void = std::ptr::null_mut();
            let rc = unsafe { (api.source_create_pixelmap)(src, opts, &mut pm) };
            if rc != 0 || pm.is_null() {
                return Err(format!("decode failed: {}", err_message(rc)));
            }
            Ok(pm)
        })();
        unsafe { (api.dec_release)(opts) };
        let pm = decoded?;

        let out = encode_jpeg(pm, quality).map(|(b64, bytes, w, h)| {
            format!(
                "{{\"path\":{},\"sourceWidth\":{},\"sourceHeight\":{},\
                 \"width\":{},\"height\":{},\"jpegBytes\":{},\"dataUri\":{}}}",
                json_str(path),
                sw,
                sh,
                w,
                h,
                bytes,
                json_str(&format!("data:image/jpeg;base64,{b64}"))
            )
        });
        unsafe { (api.pm_release)(pm) };
        out
    })
}

/// Pack a pixel map to JPEG. Returns (base64, byte count, width, height).
fn encode_jpeg(pm: *mut c_void, quality: u32) -> Result<(String, usize, u32, u32), String> {
    let api = image_api().ok_or(NO_IMAGE)?;
    let (mut w, mut h) = (0u32, 0u32);
    let mut info: *mut c_void = std::ptr::null_mut();
    if unsafe { (api.pm_info_create)(&mut info) } == 0 && !info.is_null() {
        unsafe {
            (api.pm_get_info)(pm, info);
            (api.pm_info_width)(info, &mut w);
            (api.pm_info_height)(info, &mut h);
            (api.pm_info_release)(info);
        }
    }

    let mut packer: *mut c_void = std::ptr::null_mut();
    if unsafe { (api.packer_create)(&mut packer) } != 0 || packer.is_null() {
        return Err("could not create an image packer".into());
    }
    let result = (|| {
        let mut opts: *mut c_void = std::ptr::null_mut();
        if unsafe { (api.pack_opts_create)(&mut opts) } != 0 || opts.is_null() {
            return Err("could not allocate packing options".to_string());
        }
        let packed = (|| {
            let mime = CString::new("image/jpeg").map_err(|_| "bad mime")?;
            let mut ms = ImageString {
                data: mime.as_ptr() as *mut c_char,
                size: "image/jpeg".len(),
            };
            unsafe {
                (api.pack_opts_mime)(opts, &mut ms);
                (api.pack_opts_quality)(opts, quality);
            }
            // The packer writes into a caller-provided buffer and reports how
            // much it used. A thumbnail bounded at 1024px cannot plausibly
            // exceed this even at quality 100.
            let mut buf = vec![0u8; 4 * 1024 * 1024];
            let mut size = buf.len();
            let rc = unsafe {
                (api.packer_from_pixelmap)(packer, opts, pm, buf.as_mut_ptr(), &mut size)
            };
            if rc != 0 {
                return Err(format!("encode failed: {}", err_message(rc)));
            }
            buf.truncate(size);
            Ok((b64(&buf), size))
        })();
        unsafe { (api.pack_opts_release)(opts) };
        packed
    })();
    unsafe { (api.packer_release)(packer) };
    result.map(|(s, n)| (s, n, w, h))
}

fn b64(bytes: &[u8]) -> String {
    const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for c in bytes.chunks(3) {
        let b = [c[0], *c.get(1).unwrap_or(&0), *c.get(2).unwrap_or(&0)];
        let n = ((b[0] as u32) << 16) | ((b[1] as u32) << 8) | b[2] as u32;
        out.push(T[(n >> 18 & 63) as usize] as char);
        out.push(T[(n >> 12 & 63) as usize] as char);
        out.push(if c.len() > 1 {
            T[(n >> 6 & 63) as usize] as char
        } else {
            '='
        });
        out.push(if c.len() > 2 {
            T[(n & 63) as usize] as char
        } else {
            '='
        });
    }
    out
}

// ---------------------------------------------------------------------------
// Camera enumeration.
//
// Listing only. Capture is a session pipeline -- input, preview output, photo
// output, each bound to a surface -- and needs ohos.permission.CAMERA, a
// user_grant. Enumeration needs neither, so it is worth having on its own:
// it answers what the hardware is before anything asks for permission to use it.
// ---------------------------------------------------------------------------

#[repr(C)]
struct CameraDevice {
    camera_id: *mut c_char,
    position: i32,
    camera_type: i32,
    connection: i32,
}

fn position_name(p: i32) -> &'static str {
    match p {
        1 => "back",
        2 => "front",
        _ => "unspecified",
    }
}

fn type_name(t: i32) -> &'static str {
    match t {
        1 => "wide-angle",
        2 => "ultra-wide",
        3 => "telephoto",
        4 => "true-depth",
        _ => "default",
    }
}

fn connection_name(c: i32) -> &'static str {
    match c {
        1 => "usb",
        2 => "remote",
        _ => "built-in",
    }
}

/// The cameras this device has. JSON array.
pub fn cameras() -> Result<String, String> {
    let api = camera_api().ok_or(NO_CAMERA)?;
    let mut mgr: *mut c_void = std::ptr::null_mut();
    let rc = unsafe { (api.get_manager)(&mut mgr) };
    if rc != 0 || mgr.is_null() {
        return Err(match rc {
            201 => "permission denied (ohos.permission.CAMERA)".into(),
            _ => format!("camera manager unavailable ({rc})"),
        });
    }

    let result = (|| {
        let api = camera_api().ok_or(NO_CAMERA)?;
        let mut list: *mut CameraDevice = std::ptr::null_mut();
        let mut n: u32 = 0;
        let rc = unsafe { (api.supported)(mgr, &mut list, &mut n) };
        if rc != 0 || list.is_null() {
            return Err(format!("could not enumerate cameras ({rc})"));
        }
        let mut out = Vec::new();
        for i in 0..n as isize {
            let d = unsafe { &*list.offset(i) };
            let id = if d.camera_id.is_null() {
                String::new()
            } else {
                unsafe { std::ffi::CStr::from_ptr(d.camera_id) }
                    .to_string_lossy()
                    .into_owned()
            };
            out.push(format!(
                "{{\"id\":{},\"position\":{},\"type\":{},\"connection\":{}}}",
                json_str(&id),
                json_str(position_name(d.position)),
                json_str(type_name(d.camera_type)),
                json_str(connection_name(d.connection))
            ));
        }
        unsafe { (api.del_supported)(mgr, list, n) };
        Ok(format!("[{}]", out.join(",")))
    })();

    unsafe { (api.del_manager)(mgr) };
    result
}

// ---------------------------------------------------------------------------
// Camera preview.
//
// The session pipeline, bound to a surface Rust made itself (see xcomp.rs).
// ArkTS is nowhere on this path: ARKUI_NODE_XCOMPONENT is a real node type, so
// the surface is a node in the same native tree as everything else, and the
// camera writes frames straight into it. No per-frame crossing, and no
// per-rebuild crossing either -- which is what separates this from the web
// slots, where the absence of ARKUI_NODE_WEB forces an ArkTS overlay.
// ---------------------------------------------------------------------------

#[repr(C)]
struct CameraSize {
    width: u32,
    height: u32,
}

#[repr(C)]
struct CameraProfile {
    format: i32,
    size: CameraSize,
}

#[repr(C)]
struct OutputCapability {
    preview_profiles: *mut *mut CameraProfile,
    preview_profiles_size: u32,
    photo_profiles: *mut *mut CameraProfile,
    photo_profiles_size: u32,
    video_profiles: *mut *mut c_void,
    video_profiles_size: u32,
    supported_metadata_object_types: *mut *mut c_void,
    metadata_types_size: u32,
}

struct SessionApi {
    get_capability:
        unsafe extern "C" fn(*mut c_void, *const CameraDevice, *mut *mut OutputCapability) -> i32,
    del_capability: unsafe extern "C" fn(*mut c_void, *mut OutputCapability) -> i32,
    create_input: unsafe extern "C" fn(*mut c_void, *const CameraDevice, *mut *mut c_void) -> i32,
    input_open: unsafe extern "C" fn(*mut c_void) -> i32,
    input_release: unsafe extern "C" fn(*mut c_void) -> i32,
    create_session: unsafe extern "C" fn(*mut c_void, *mut *mut c_void) -> i32,
    begin_config: unsafe extern "C" fn(*mut c_void) -> i32,
    add_input: unsafe extern "C" fn(*mut c_void, *mut c_void) -> i32,
    create_preview: unsafe extern "C" fn(
        *mut c_void,
        *const CameraProfile,
        *const c_char,
        *mut *mut c_void,
    ) -> i32,
    add_preview: unsafe extern "C" fn(*mut c_void, *mut c_void) -> i32,
    commit_config: unsafe extern "C" fn(*mut c_void) -> i32,
    session_start: unsafe extern "C" fn(*mut c_void) -> i32,
    session_stop: unsafe extern "C" fn(*mut c_void) -> i32,
    session_release: unsafe extern "C" fn(*mut c_void) -> i32,
}

static SESSION_API: std::sync::OnceLock<Option<SessionApi>> = std::sync::OnceLock::new();

fn session_api() -> Option<&'static SessionApi> {
    SESSION_API
        .get_or_init(|| {
            let h = open_any(&["libohcamera.so"])?;
            let hs = [h];
            Some(SessionApi {
                get_capability: sym(&hs, "OH_CameraManager_GetSupportedCameraOutputCapability")?,
                del_capability: sym(
                    &hs,
                    "OH_CameraManager_DeleteSupportedCameraOutputCapability",
                )?,
                create_input: sym(&hs, "OH_CameraManager_CreateCameraInput")?,
                input_open: sym(&hs, "OH_CameraInput_Open")?,
                input_release: sym(&hs, "OH_CameraInput_Release")?,
                create_session: sym(&hs, "OH_CameraManager_CreateCaptureSession")?,
                begin_config: sym(&hs, "OH_CaptureSession_BeginConfig")?,
                add_input: sym(&hs, "OH_CaptureSession_AddInput")?,
                create_preview: sym(&hs, "OH_CameraManager_CreatePreviewOutput")?,
                add_preview: sym(&hs, "OH_CaptureSession_AddPreviewOutput")?,
                commit_config: sym(&hs, "OH_CaptureSession_CommitConfig")?,
                session_start: sym(&hs, "OH_CaptureSession_Start")?,
                session_stop: sym(&hs, "OH_CaptureSession_Stop")?,
                session_release: sym(&hs, "OH_CaptureSession_Release")?,
            })
        })
        .as_ref()
}

/// A running preview. Held so it can be stopped, and because releasing the
/// session while it is running is how you get a black surface and a leaked
/// camera that no other app can then open.
struct Preview {
    manager: *mut c_void,
    input: *mut c_void,
    session: *mut c_void,
    output: *mut c_void,
}
// The handles are only ever touched under RUNNING's lock, on one worker at a
// time. Rust cannot see that through the raw pointers, so it is asserted here.
unsafe impl Send for Preview {}

static RUNNING: Mutex<Option<Preview>> = Mutex::new(None);

fn cam_error(rc: i32) -> String {
    match rc {
        7_400_101 => "invalid argument".into(),
        7_400_102 => "operation not allowed in this session state".into(),
        7_400_103 => "session not configured".into(),
        // Do not trust this one's name. The camera service logs
        //   HCameraService::CheckPermission: Permission to Access Camera Denied
        //   CreateCameraDevice Check OHOS_PERMISSION_CAMERA fail 15
        // and the NDK still surfaces 7400201, whose documented meaning is "in
        // use by another application". A missing CAMERA permission and a
        // genuine conflict are indistinguishable here, so the message says so
        // rather than sending the caller after the wrong fix.
        7_400_201 => "camera unavailable — either in use by another app, or \
                      ohos.permission.CAMERA is not granted"
            .into(),
        7_400_202 => "camera disabled by device policy".into(),
        7_400_203 => "the camera is already in use here".into(),
        201 => "permission denied (ohos.permission.CAMERA)".into(),
        _ => format!("camera error ({rc})"),
    }
}

/// Choose a preview profile near the surface, so the camera is not asked to
/// stream 4K into a 400 px box. Nearest by area rather than exact match: the
/// supported list is discrete and rarely contains the size actually wanted.
fn pick_profile(cap: &OutputCapability, want_w: u32, want_h: u32) -> Option<*mut CameraProfile> {
    if cap.preview_profiles.is_null() || cap.preview_profiles_size == 0 {
        return None;
    }
    let want = (want_w as f64) * (want_h as f64);
    let mut best: Option<*mut CameraProfile> = None;
    let mut best_d = f64::MAX;
    for i in 0..cap.preview_profiles_size as isize {
        let p = unsafe { *cap.preview_profiles.offset(i) };
        if p.is_null() {
            continue;
        }
        let s = unsafe { &(*p).size };
        let d = ((s.width as f64) * (s.height as f64) - want).abs();
        if d < best_d {
            best_d = d;
            best = Some(p);
        }
    }
    best
}

/// Start the preview into `surface_id`. Idempotent-ish: an already-running
/// preview is stopped first, because two sessions on one camera is the state
/// that produces 7400203 and a surface that never lights up.
pub fn preview_start(
    surface_id: u64,
    want_w: u32,
    want_h: u32,
    front: bool,
) -> Result<String, String> {
    let api = camera_api().ok_or(NO_CAMERA)?;
    let sapi = session_api().ok_or(NO_CAMERA)?;
    preview_stop().ok();

    let mut mgr: *mut c_void = std::ptr::null_mut();
    let rc = unsafe { (api.get_manager)(&mut mgr) };
    if rc != 0 || mgr.is_null() {
        return Err(cam_error(rc));
    }

    // Everything below must unwind on failure or the camera stays held.
    let built = (|| -> Result<(Preview, String), String> {
        let mut list: *mut CameraDevice = std::ptr::null_mut();
        let mut n: u32 = 0;
        let rc = unsafe { (api.supported)(mgr, &mut list, &mut n) };
        if rc != 0 || list.is_null() || n == 0 {
            return Err(cam_error(rc));
        }
        let want_pos = if front { 2 } else { 1 };
        let mut chosen: isize = -1;
        for i in 0..n as isize {
            if unsafe { (*list.offset(i)).position } == want_pos {
                chosen = i;
                break;
            }
        }
        if chosen < 0 {
            chosen = 0;
        }
        let device = unsafe { list.offset(chosen) };

        let mut cap: *mut OutputCapability = std::ptr::null_mut();
        let rc = unsafe { (sapi.get_capability)(mgr, device, &mut cap) };
        if rc != 0 || cap.is_null() {
            return Err(format!("output capability: {}", cam_error(rc)));
        }
        let profile = pick_profile(unsafe { &*cap }, want_w, want_h);
        let Some(profile) = profile else {
            unsafe { (sapi.del_capability)(mgr, cap) };
            return Err("no preview profiles offered".into());
        };
        let (pw, ph) = unsafe { ((*profile).size.width, (*profile).size.height) };

        let mut input: *mut c_void = std::ptr::null_mut();
        let rc = unsafe { (sapi.create_input)(mgr, device, &mut input) };
        if rc != 0 || input.is_null() {
            unsafe { (sapi.del_capability)(mgr, cap) };
            return Err(format!("camera input: {}", cam_error(rc)));
        }
        let rc = unsafe { (sapi.input_open)(input) };
        if rc != 0 {
            unsafe {
                (sapi.input_release)(input);
                (sapi.del_capability)(mgr, cap);
            }
            return Err(format!("open camera: {}", cam_error(rc)));
        }

        // The surface id crosses as a decimal string -- the camera API takes
        // `const char*`, not the u64 the window handed back.
        let sid = CString::new(surface_id.to_string()).map_err(|_| "bad surface id")?;
        let mut output: *mut c_void = std::ptr::null_mut();
        let rc = unsafe { (sapi.create_preview)(mgr, profile, sid.as_ptr(), &mut output) };
        unsafe { (sapi.del_capability)(mgr, cap) };
        if rc != 0 || output.is_null() {
            unsafe { (sapi.input_release)(input) };
            return Err(format!("preview output: {}", cam_error(rc)));
        }

        let mut session: *mut c_void = std::ptr::null_mut();
        let rc = unsafe { (sapi.create_session)(mgr, &mut session) };
        if rc != 0 || session.is_null() {
            unsafe { (sapi.input_release)(input) };
            return Err(format!("session: {}", cam_error(rc)));
        }

        // Order matters: begin, add, commit, start. Adding outside a config
        // block returns 7400102 rather than doing anything.
        let steps: [(&str, i32); 5] = [
            ("beginConfig", unsafe { (sapi.begin_config)(session) }),
            ("addInput", unsafe { (sapi.add_input)(session, input) }),
            ("addPreview", unsafe { (sapi.add_preview)(session, output) }),
            ("commitConfig", unsafe { (sapi.commit_config)(session) }),
            ("start", unsafe { (sapi.session_start)(session) }),
        ];
        for (what, rc) in steps {
            if rc != 0 {
                unsafe {
                    (sapi.session_release)(session);
                    (sapi.input_release)(input);
                }
                return Err(format!("{what}: {}", cam_error(rc)));
            }
        }

        Ok((
            Preview {
                manager: mgr,
                input,
                session,
                output,
            },
            format!(
                "{{\"running\":true,\"surfaceId\":\"{surface_id}\",\"previewWidth\":{pw},\
                 \"previewHeight\":{ph},\"camera\":\"{}\"}}",
                if front { "front" } else { "back" }
            ),
        ))
    })();

    match built {
        Ok((p, json)) => {
            if let Ok(mut r) = RUNNING.lock() {
                *r = Some(p);
            }
            Ok(json)
        }
        Err(e) => {
            unsafe { (api.del_manager)(mgr) };
            Err(e)
        }
    }
}

/// Stop and release, in the order that does not leave the camera held.
pub fn preview_stop() -> Result<String, String> {
    let api = camera_api().ok_or(NO_CAMERA)?;
    let sapi = session_api().ok_or(NO_CAMERA)?;
    let Some(p) = RUNNING.lock().ok().and_then(|mut r| r.take()) else {
        return Ok("{\"running\":false}".into());
    };
    unsafe {
        (sapi.session_stop)(p.session);
        (sapi.session_release)(p.session);
        (sapi.input_release)(p.input);
        (api.del_manager)(p.manager);
    }
    let _ = p.output;
    Ok("{\"running\":false}".into())
}
