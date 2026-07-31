//! Screen capture, from `OH_NativeDisplayManager_CaptureScreenPixelmap`.
//!
//! This one exists to answer a question rather than to power a card: can an
//! ordinary application read the framebuffer? On Android the equivalent needs
//! `READ_FRAME_BUFFER`, which the shell user has and an app does not — so
//! `screencap` works over adb and the same call fails inside an app.
//!
//! # The answer, measured
//!
//! OpenHarmony draws the line in the same place, and harder. Calling this
//! without the permission returns 201, which the card reports as
//!
//! > permission denied (ohos.permission.CAPTURE_SCREEN)
//!
//! and *declaring* the permission does not help — the HAP will not install:
//!
//! ```text
//! code:9568289 install failed due to grant request permissions failed.
//!              PermissionName: ohos.permission.CAPTURE_SCREEN
//! ```
//!
//! So this is not a runtime grant an app can ask for and be refused; it is
//! above this application's privilege level entirely, and a build declaring it
//! is rejected at install. Any design that wanted an app to screenshot itself —
//! an agent scoring its own UI, say — needs a system-signed component or an
//! external capture over hdc. The code stays because the refusal is the useful
//! part: it is measured here rather than assumed, and it will start returning
//! frames the day it runs somewhere with the privilege.
//!
//! # Why it does not return the image
//!
//! A frame on this panel is 1320 × 2760 × 4 bytes ≈ 14 MB. The bridge moves
//! replies as one JSON string, which is then evaluated into the page by
//! `runJavaScript`, so a frame would be paid for several times over and base64
//! would add a third again. Moving screenshots wants a shared buffer or a file
//! handle, not this channel.
//!
//! What it returns instead is the frame's geometry plus the **average colour of
//! the pixels actually read**. That is deliberately not metadata alone: the
//! dimensions could be reported by a call that captured nothing, whereas an
//! average requires walking the buffer, so a plausible colour is evidence the
//! pixels were really there.

use crate::bridge::json_str;
use std::ffi::c_void;

extern "C" {
    fn OH_NativeDisplayManager_GetDefaultDisplayId(id: *mut u64) -> i32;
    fn OH_NativeDisplayManager_CaptureScreenPixelmap(
        display_id: u32,
        pixelmap: *mut *mut c_void,
    ) -> i32;

    fn OH_PixelmapImageInfo_Create(info: *mut *mut c_void) -> i32;
    fn OH_PixelmapImageInfo_GetWidth(info: *mut c_void, w: *mut u32) -> i32;
    fn OH_PixelmapImageInfo_GetHeight(info: *mut c_void, h: *mut u32) -> i32;
    fn OH_PixelmapImageInfo_GetRowStride(info: *mut c_void, s: *mut u32) -> i32;
    fn OH_PixelmapImageInfo_GetPixelFormat(info: *mut c_void, f: *mut i32) -> i32;
    fn OH_PixelmapImageInfo_Release(info: *mut c_void) -> i32;
    fn OH_PixelmapNative_GetImageInfo(pm: *mut c_void, info: *mut c_void) -> i32;
    fn OH_PixelmapNative_AccessPixels(pm: *mut c_void, addr: *mut *mut c_void) -> i32;
    fn OH_PixelmapNative_UnaccessPixels(pm: *mut c_void) -> i32;
    fn OH_PixelmapNative_Release(pm: *mut c_void) -> i32;
}

fn format_name(f: i32) -> &'static str {
    match f {
        1 => "ARGB_8888",
        2 => "RGB_565",
        3 => "RGBA_8888",
        4 => "BGRA_8888",
        5 => "RGB_888",
        6 => "ALPHA_8",
        7 => "RGBA_F16",
        8 => "NV21",
        9 => "NV12",
        _ => "unknown",
    }
}

/// Capture the default display and describe it. JSON object, or an error.
pub fn screen() -> Result<String, String> {
    let mut display_id: u64 = 0;
    if unsafe { OH_NativeDisplayManager_GetDefaultDisplayId(&mut display_id) } != 0 {
        return Err("could not resolve the default display".into());
    }

    let mut pm: *mut c_void = std::ptr::null_mut();
    let rc = unsafe { OH_NativeDisplayManager_CaptureScreenPixelmap(display_id as u32, &mut pm) };
    if rc != 0 || pm.is_null() {
        // 201 is the NDK's no-permission code. Naming it matters: "denied" and
        // "unsupported on this build" are different answers to the question
        // this module exists to ask.
        return Err(match rc {
            201 => "permission denied (ohos.permission.CAPTURE_SCREEN)".into(),
            801 => "capability not supported on this device".into(),
            _ => format!("capture failed ({rc})"),
        });
    }

    let result = (|| {
        let mut info: *mut c_void = std::ptr::null_mut();
        if unsafe { OH_PixelmapImageInfo_Create(&mut info) } != 0 || info.is_null() {
            return Err("could not allocate image info".to_string());
        }
        let out = (|| {
            if unsafe { OH_PixelmapNative_GetImageInfo(pm, info) } != 0 {
                return Err("could not read image info".to_string());
            }
            let mut w = 0u32;
            let mut h = 0u32;
            let mut stride = 0u32;
            let mut fmt = 0i32;
            unsafe {
                OH_PixelmapImageInfo_GetWidth(info, &mut w);
                OH_PixelmapImageInfo_GetHeight(info, &mut h);
                OH_PixelmapImageInfo_GetRowStride(info, &mut stride);
                OH_PixelmapImageInfo_GetPixelFormat(info, &mut fmt);
            }

            // Walk the buffer for an average, so the answer is evidence the
            // pixels were readable rather than a description of a frame that
            // might not have arrived.
            let mut avg = "null".to_string();
            let mut addr: *mut c_void = std::ptr::null_mut();
            if unsafe { OH_PixelmapNative_AccessPixels(pm, &mut addr) } == 0 && !addr.is_null() {
                let bpp = match fmt {
                    2 => 2usize, // RGB_565
                    5 => 3,      // RGB_888
                    6 => 1,      // ALPHA_8
                    _ => 4,
                };
                if bpp == 4 && stride >= w * 4 {
                    let (mut r, mut g, mut b, mut n) = (0u64, 0u64, 0u64, 0u64);
                    // Every 16th pixel of every 16th row: 256x fewer reads, and
                    // an average does not need every sample.
                    let base = addr as *const u8;
                    for y in (0..h).step_by(16) {
                        for x in (0..w).step_by(16) {
                            let off = (y as usize) * stride as usize + (x as usize) * 4;
                            let p = unsafe { std::slice::from_raw_parts(base.add(off), 4) };
                            // RGBA_8888 and BGRA_8888 differ in channel order;
                            // name them right rather than reporting BGR as RGB.
                            let (rr, gg, bb) = if fmt == 4 {
                                (p[2], p[1], p[0])
                            } else {
                                (p[0], p[1], p[2])
                            };
                            r += rr as u64;
                            g += gg as u64;
                            b += bb as u64;
                            n += 1;
                        }
                    }
                    if n > 0 {
                        avg = format!(
                            "\"#{:02x}{:02x}{:02x}\"",
                            (r / n) as u8,
                            (g / n) as u8,
                            (b / n) as u8
                        );
                    }
                }
                unsafe { OH_PixelmapNative_UnaccessPixels(pm) };
            }

            Ok(format!(
                "{{\"width\":{},\"height\":{},\"rowStride\":{},\"pixelFormat\":{},\
                 \"bytes\":{},\"averageColor\":{}}}",
                w,
                h,
                stride,
                json_str(format_name(fmt)),
                stride as u64 * h as u64,
                avg
            ))
        })();
        unsafe { OH_PixelmapImageInfo_Release(info) };
        out
    })();

    unsafe { OH_PixelmapNative_Release(pm) };
    result
}
