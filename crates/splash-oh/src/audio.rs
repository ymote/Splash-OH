//! Audio playback and capture, from `libohaudio`.
//!
//! The one large media kit this device does not block. The image codecs are
//! absent here (the `*Native*` symbol family simply is not exported) and video
//! would need them — but PCM needs no codec at all, so a tone can be
//! synthesised and a microphone read with nothing but the stream API.
//!
//! # Both directions are callback-driven, on the audio thread
//!
//! There is no "write this buffer" call. `OH_AudioRenderer_OnWriteData` is
//! invoked by the audio service on **its own real-time thread** whenever the
//! device wants more samples, and capture is the mirror image. So neither
//! direction can be a blocking function that returns the answer; both are a
//! start, a wait, and a stop, with the callback filling in a static.
//!
//! That thread is the reason these callbacks do as little as possible: filling
//! a sine wave and accumulating a sum of squares is arithmetic over the buffer
//! it was handed. Locking, allocating, or logging inside them is how you get an
//! audible glitch.

use crate::bridge::json_str;
use std::ffi::c_void;
use std::sync::atomic::{AtomicBool, AtomicI64, AtomicU32, AtomicU64, Ordering};
use std::sync::Mutex;

const AUDIOSTREAM_TYPE_RENDERER: i32 = 1;
const AUDIOSTREAM_TYPE_CAPTURER: i32 = 2;
const AUDIOSTREAM_SAMPLE_S16LE: i32 = 1;
const SAMPLE_RATE: i32 = 48_000;
const CHANNELS: i32 = 1;

#[repr(C)]
struct RendererCallbacks {
    on_write_data: extern "C" fn(*mut c_void, *mut c_void, *mut c_void, i32) -> i32,
    on_stream_event: extern "C" fn(*mut c_void, *mut c_void, i32) -> i32,
    on_interrupt_event: extern "C" fn(*mut c_void, *mut c_void, i32, i32) -> i32,
    on_error: extern "C" fn(*mut c_void, *mut c_void, i32) -> i32,
}

#[repr(C)]
struct CapturerCallbacks {
    on_read_data: extern "C" fn(*mut c_void, *mut c_void, *mut c_void, i32) -> i32,
    on_stream_event: extern "C" fn(*mut c_void, *mut c_void, i32) -> i32,
    on_interrupt_event: extern "C" fn(*mut c_void, *mut c_void, i32, i32) -> i32,
    on_error: extern "C" fn(*mut c_void, *mut c_void, i32) -> i32,
}

extern "C" {
    fn dlopen(file: *const std::os::raw::c_char, mode: i32) -> *mut c_void;
    fn dlsym(handle: *mut c_void, name: *const std::os::raw::c_char) -> *mut c_void;
}

/// Resolved at runtime rather than linked, for the reason #26 established: an
/// unresolved `DT_NEEDED` symbol is fatal to the whole library and would take
/// every unrelated capability down with it.
struct AudioApi {
    builder_create: unsafe extern "C" fn(*mut *mut c_void, i32) -> i32,
    builder_destroy: unsafe extern "C" fn(*mut c_void) -> i32,
    set_rate: unsafe extern "C" fn(*mut c_void, i32) -> i32,
    set_channels: unsafe extern "C" fn(*mut c_void, i32) -> i32,
    set_format: unsafe extern "C" fn(*mut c_void, i32) -> i32,
    set_renderer_cb: unsafe extern "C" fn(*mut c_void, RendererCallbacks, *mut c_void) -> i32,
    set_capturer_cb: unsafe extern "C" fn(*mut c_void, CapturerCallbacks, *mut c_void) -> i32,
    gen_renderer: unsafe extern "C" fn(*mut c_void, *mut *mut c_void) -> i32,
    gen_capturer: unsafe extern "C" fn(*mut c_void, *mut *mut c_void) -> i32,
    renderer_start: unsafe extern "C" fn(*mut c_void) -> i32,
    renderer_stop: unsafe extern "C" fn(*mut c_void) -> i32,
    renderer_release: unsafe extern "C" fn(*mut c_void) -> i32,
    capturer_start: unsafe extern "C" fn(*mut c_void) -> i32,
    capturer_stop: unsafe extern "C" fn(*mut c_void) -> i32,
    capturer_release: unsafe extern "C" fn(*mut c_void) -> i32,
}

static AUDIO_API: std::sync::OnceLock<Option<AudioApi>> = std::sync::OnceLock::new();

fn sym<T>(h: *mut c_void, name: &str) -> Option<T> {
    let c = std::ffi::CString::new(name).ok()?;
    let p = unsafe { dlsym(h, c.as_ptr()) };
    if p.is_null() {
        crate::log(&format!("audio: symbol {name} not found"));
        return None;
    }
    Some(unsafe { std::mem::transmute_copy(&p) })
}

fn api() -> Option<&'static AudioApi> {
    AUDIO_API
        .get_or_init(|| {
            let name = std::ffi::CString::new("libohaudio.so").ok()?;
            let h = unsafe { dlopen(name.as_ptr(), 2) };
            if h.is_null() {
                crate::log("audio: libohaudio.so did not load");
                return None;
            }
            Some(AudioApi {
                builder_create: sym(h, "OH_AudioStreamBuilder_Create")?,
                builder_destroy: sym(h, "OH_AudioStreamBuilder_Destroy")?,
                set_rate: sym(h, "OH_AudioStreamBuilder_SetSamplingRate")?,
                set_channels: sym(h, "OH_AudioStreamBuilder_SetChannelCount")?,
                set_format: sym(h, "OH_AudioStreamBuilder_SetSampleFormat")?,
                set_renderer_cb: sym(h, "OH_AudioStreamBuilder_SetRendererCallback")?,
                set_capturer_cb: sym(h, "OH_AudioStreamBuilder_SetCapturerCallback")?,
                gen_renderer: sym(h, "OH_AudioStreamBuilder_GenerateRenderer")?,
                gen_capturer: sym(h, "OH_AudioStreamBuilder_GenerateCapturer")?,
                renderer_start: sym(h, "OH_AudioRenderer_Start")?,
                renderer_stop: sym(h, "OH_AudioRenderer_Stop")?,
                renderer_release: sym(h, "OH_AudioRenderer_Release")?,
                capturer_start: sym(h, "OH_AudioCapturer_Start")?,
                capturer_stop: sym(h, "OH_AudioCapturer_Stop")?,
                capturer_release: sym(h, "OH_AudioCapturer_Release")?,
            })
        })
        .as_ref()
}

const NO_AUDIO: &str = "audio kit unavailable on this device";

fn audio_error(rc: i32) -> String {
    match rc {
        1 => "invalid parameter".into(),
        2 => "illegal state".into(),
        6_800_301 => "system error".into(),
        _ => format!("audio error ({rc})"),
    }
}

// --- playback ---------------------------------------------------------------

/// Phase accumulator for the tone, in samples. An `AtomicU64` rather than a
/// lock because the audio thread reads and writes it every callback and must
/// never wait on anything.
static TONE_PHASE: AtomicU64 = AtomicU64::new(0);
static TONE_HZ: AtomicU32 = AtomicU32::new(440);
static FRAMES_WRITTEN: AtomicU64 = AtomicU64::new(0);
static CALLBACKS_SEEN: AtomicU64 = AtomicU64::new(0);

extern "C" fn on_write(
    _r: *mut c_void,
    _user: *mut c_void,
    buffer: *mut c_void,
    length: i32,
) -> i32 {
    if buffer.is_null() || length <= 0 {
        return 0;
    }
    let frames = (length as usize) / 2; // S16LE, mono
    let hz = TONE_HZ.load(Ordering::Relaxed) as f64;
    let mut phase = TONE_PHASE.load(Ordering::Relaxed);
    let out = unsafe { std::slice::from_raw_parts_mut(buffer as *mut i16, frames) };
    for s in out.iter_mut() {
        let t = phase as f64 / SAMPLE_RATE as f64;
        // A third of full scale: loud enough to hear and to measure, quiet
        // enough not to be unpleasant if something loops it.
        *s = ((t * hz * std::f64::consts::TAU).sin() * 10_000.0) as i16;
        phase += 1;
    }
    TONE_PHASE.store(phase, Ordering::Relaxed);
    FRAMES_WRITTEN.fetch_add(frames as u64, Ordering::Relaxed);
    CALLBACKS_SEEN.fetch_add(1, Ordering::Relaxed);
    0
}

extern "C" fn on_event(_a: *mut c_void, _u: *mut c_void, _e: i32) -> i32 {
    0
}
extern "C" fn on_interrupt(_a: *mut c_void, _u: *mut c_void, _t: i32, _h: i32) -> i32 {
    0
}
extern "C" fn on_error(_a: *mut c_void, _u: *mut c_void, _e: i32) -> i32 {
    0
}

/// Play a tone for `ms`. Reports how many frames the audio service actually
/// consumed, which is the part worth checking: a stream that starts and is
/// never asked for samples is silent, and only the frame count tells them apart.
pub fn tone(hz: u32, ms: u64) -> Result<String, String> {
    let api = api().ok_or(NO_AUDIO)?;
    let ms = ms.clamp(50, 3000);
    TONE_HZ.store(hz.clamp(50, 12_000), Ordering::Relaxed);
    TONE_PHASE.store(0, Ordering::Relaxed);
    FRAMES_WRITTEN.store(0, Ordering::Relaxed);
    CALLBACKS_SEEN.store(0, Ordering::Relaxed);

    let mut builder: *mut c_void = std::ptr::null_mut();
    let rc = unsafe { (api.builder_create)(&mut builder, AUDIOSTREAM_TYPE_RENDERER) };
    if rc != 0 || builder.is_null() {
        return Err(audio_error(rc));
    }

    let played = (|| -> Result<(u64, u64), String> {
        unsafe {
            (api.set_rate)(builder, SAMPLE_RATE);
            (api.set_channels)(builder, CHANNELS);
            (api.set_format)(builder, AUDIOSTREAM_SAMPLE_S16LE);
        }
        let cbs = RendererCallbacks {
            on_write_data: on_write,
            on_stream_event: on_event,
            on_interrupt_event: on_interrupt,
            on_error,
        };
        // Passed by value, so unlike the XComponent table there is nothing to
        // keep alive here -- the kit copies it.
        let rc = unsafe { (api.set_renderer_cb)(builder, cbs, std::ptr::null_mut()) };
        if rc != 0 {
            return Err(format!("callbacks: {}", audio_error(rc)));
        }
        let mut renderer: *mut c_void = std::ptr::null_mut();
        let rc = unsafe { (api.gen_renderer)(builder, &mut renderer) };
        if rc != 0 || renderer.is_null() {
            return Err(format!("renderer: {}", audio_error(rc)));
        }
        let rc = unsafe { (api.renderer_start)(renderer) };
        if rc != 0 {
            unsafe { (api.renderer_release)(renderer) };
            return Err(format!("start: {}", audio_error(rc)));
        }
        std::thread::sleep(std::time::Duration::from_millis(ms));
        unsafe {
            (api.renderer_stop)(renderer);
            (api.renderer_release)(renderer);
        }
        Ok((
            FRAMES_WRITTEN.load(Ordering::Relaxed),
            CALLBACKS_SEEN.load(Ordering::Relaxed),
        ))
    })();

    unsafe { (api.builder_destroy)(builder) };
    let (frames, calls) = played?;
    Ok(format!(
        "{{\"hz\":{},\"ms\":{},\"frames\":{},\"callbacks\":{},\"seconds\":{:.2}}}",
        TONE_HZ.load(Ordering::Relaxed),
        ms,
        frames,
        calls,
        frames as f64 / SAMPLE_RATE as f64
    ))
}

// --- capture ----------------------------------------------------------------

/// Sum of squares and peak, scaled to integers so the audio thread never
/// touches a lock or a float atomic.
static CAP_SUMSQ: AtomicI64 = AtomicI64::new(0);
static CAP_FRAMES: AtomicU64 = AtomicU64::new(0);
static CAP_PEAK: AtomicI64 = AtomicI64::new(0);
static CAP_ACTIVE: AtomicBool = AtomicBool::new(false);
/// One capture at a time; two would interleave into the same accumulators.
static CAPTURING: Mutex<()> = Mutex::new(());

extern "C" fn on_read(
    _c: *mut c_void,
    _user: *mut c_void,
    buffer: *mut c_void,
    length: i32,
) -> i32 {
    if buffer.is_null() || length <= 0 || !CAP_ACTIVE.load(Ordering::Relaxed) {
        return 0;
    }
    let frames = (length as usize) / 2;
    let input = unsafe { std::slice::from_raw_parts(buffer as *const i16, frames) };
    let mut sumsq: i64 = 0;
    let mut peak: i64 = 0;
    for &s in input {
        let v = s as i64;
        sumsq += v * v;
        let a = v.abs();
        if a > peak {
            peak = a;
        }
    }
    CAP_SUMSQ.fetch_add(sumsq, Ordering::Relaxed);
    CAP_FRAMES.fetch_add(frames as u64, Ordering::Relaxed);
    CAP_PEAK.fetch_max(peak, Ordering::Relaxed);
    0
}

/// Record for `ms` and report the level. Needs `ohos.permission.MICROPHONE`.
///
/// Returns RMS and peak in dBFS rather than raw counts, because the useful
/// question is "is anything arriving" and a level in dB answers it at a glance:
/// digital silence is -inf, a quiet room lands around -60, speech well above.
pub fn record_level(ms: u64) -> Result<String, String> {
    let api = api().ok_or(NO_AUDIO)?;
    let _one = CAPTURING.lock().map_err(|_| "capture lock poisoned")?;
    let ms = ms.clamp(100, 5000);

    let mut builder: *mut c_void = std::ptr::null_mut();
    let rc = unsafe { (api.builder_create)(&mut builder, AUDIOSTREAM_TYPE_CAPTURER) };
    if rc != 0 || builder.is_null() {
        return Err(audio_error(rc));
    }

    let out = (|| -> Result<(f64, f64, u64), String> {
        unsafe {
            (api.set_rate)(builder, SAMPLE_RATE);
            (api.set_channels)(builder, CHANNELS);
            (api.set_format)(builder, AUDIOSTREAM_SAMPLE_S16LE);
        }
        let cbs = CapturerCallbacks {
            on_read_data: on_read,
            on_stream_event: on_event,
            on_interrupt_event: on_interrupt,
            on_error,
        };
        let rc = unsafe { (api.set_capturer_cb)(builder, cbs, std::ptr::null_mut()) };
        if rc != 0 {
            return Err(format!("callbacks: {}", audio_error(rc)));
        }
        let mut capturer: *mut c_void = std::ptr::null_mut();
        let rc = unsafe { (api.gen_capturer)(builder, &mut capturer) };
        if rc != 0 || capturer.is_null() {
            // The mic permission failure lands here rather than at start.
            return Err(format!(
                "capturer: {} (ohos.permission.MICROPHONE may not be granted)",
                audio_error(rc)
            ));
        }

        CAP_SUMSQ.store(0, Ordering::Relaxed);
        CAP_FRAMES.store(0, Ordering::Relaxed);
        CAP_PEAK.store(0, Ordering::Relaxed);
        CAP_ACTIVE.store(true, Ordering::Relaxed);

        let rc = unsafe { (api.capturer_start)(capturer) };
        if rc != 0 {
            CAP_ACTIVE.store(false, Ordering::Relaxed);
            unsafe { (api.capturer_release)(capturer) };
            return Err(format!("start: {}", audio_error(rc)));
        }
        std::thread::sleep(std::time::Duration::from_millis(ms));
        CAP_ACTIVE.store(false, Ordering::Relaxed);
        unsafe {
            (api.capturer_stop)(capturer);
            (api.capturer_release)(capturer);
        }

        let frames = CAP_FRAMES.load(Ordering::Relaxed);
        if frames == 0 {
            return Err("no audio frames arrived".into());
        }
        let mean = CAP_SUMSQ.load(Ordering::Relaxed) as f64 / frames as f64;
        let rms = mean.sqrt();
        let peak = CAP_PEAK.load(Ordering::Relaxed) as f64;
        // Full scale for S16 is 32768.
        let db = |v: f64| {
            if v <= 0.0 {
                -120.0
            } else {
                20.0 * (v / 32768.0).log10()
            }
        };
        Ok((db(rms), db(peak), frames))
    })();

    unsafe { (api.builder_destroy)(builder) };
    let (rms_db, peak_db, frames) = out?;
    Ok(format!(
        "{{\"rmsDb\":{:.1},\"peakDb\":{:.1},\"frames\":{},\"seconds\":{:.2},\"note\":{}}}",
        rms_db,
        peak_db,
        frames,
        frames as f64 / SAMPLE_RATE as f64,
        json_str(if rms_db < -90.0 {
            "digital silence — the stream ran but carried no signal"
        } else if rms_db < -55.0 {
            "quiet room"
        } else {
            "signal present"
        })
    ))
}

// ---------------------------------------------------------------------------
// Video playback.
//
// AVPlayer takes an OHNativeWindow directly, and xcomp already has one -- so
// the decoder writes into the same surface the camera preview uses, and ArkTS
// is on neither path. That is the payoff of ARKUI_NODE_XCOMPONENT being a real
// node type: one native surface, several native producers.
//
// Local files only. AVPlayer would happily fetch a URL, but that would hand a
// page the ability to make the media stack pull anything it names -- a wider
// capability than http.get, which is allowlisted precisely to prevent that.
// ---------------------------------------------------------------------------

struct PlayerApi {
    create: unsafe extern "C" fn() -> *mut c_void,
    set_fd_source: unsafe extern "C" fn(*mut c_void, i32, i64, i64) -> i32,
    set_surface: unsafe extern "C" fn(*mut c_void, *mut c_void) -> i32,
    prepare: unsafe extern "C" fn(*mut c_void) -> i32,
    play: unsafe extern "C" fn(*mut c_void) -> i32,
    stop: unsafe extern "C" fn(*mut c_void) -> i32,
    release: unsafe extern "C" fn(*mut c_void) -> i32,
    get_state: unsafe extern "C" fn(*mut c_void, *mut i32) -> i32,
    get_duration: unsafe extern "C" fn(*mut c_void, *mut i32) -> i32,
}

static PLAYER_API: std::sync::OnceLock<Option<PlayerApi>> = std::sync::OnceLock::new();

fn player_api() -> Option<&'static PlayerApi> {
    PLAYER_API
        .get_or_init(|| {
            let name = std::ffi::CString::new("libavplayer.so").ok()?;
            let h = unsafe { dlopen(name.as_ptr(), 2) };
            if h.is_null() {
                crate::log("video: libavplayer.so did not load");
                return None;
            }
            Some(PlayerApi {
                create: sym(h, "OH_AVPlayer_Create")?,
                set_fd_source: sym(h, "OH_AVPlayer_SetFDSource")?,
                set_surface: sym(h, "OH_AVPlayer_SetVideoSurface")?,
                prepare: sym(h, "OH_AVPlayer_Prepare")?,
                play: sym(h, "OH_AVPlayer_Play")?,
                stop: sym(h, "OH_AVPlayer_Stop")?,
                release: sym(h, "OH_AVPlayer_Release")?,
                get_state: sym(h, "OH_AVPlayer_GetState")?,
                get_duration: sym(h, "OH_AVPlayer_GetDuration")?,
            })
        })
        .as_ref()
}

struct Playing {
    player: *mut c_void,
}
unsafe impl Send for Playing {}
static PLAYING: Mutex<Option<Playing>> = Mutex::new(None);

fn state_name(s: i32) -> &'static str {
    match s {
        0 => "idle",
        1 => "initialized",
        2 => "prepared",
        3 => "playing",
        4 => "paused",
        5 => "stopped",
        6 => "completed",
        7 => "released",
        8 => "error",
        _ => "unknown",
    }
}

/// Play a file from the app sandbox into the native surface.
///
/// Delivered by file descriptor rather than URL. AVPlayer accepts both, and the
/// fd form is the one that cannot be pointed at the network: the path is opened
/// here, checked to be inside the sandbox, and only the handle is handed over.
pub fn video_play(path: &str) -> Result<String, String> {
    use std::os::fd::AsRawFd;
    let api = player_api().ok_or("video player unavailable on this device")?;

    const SANDBOX: &str = "/data/storage/el2/base/";
    if !path.starts_with(SANDBOX) || path.contains("..") {
        return Err(format!("only files under {SANDBOX} may be played"));
    }
    let file = std::fs::File::open(path).map_err(|e| format!("{path}: {e}"))?;
    let size = file.metadata().map(|m| m.len()).unwrap_or(0) as i64;

    let window = crate::xcomp::window();
    if window.is_null() {
        return Err("no native surface yet".into());
    }

    video_stop().ok();

    let player = unsafe { (api.create)() };
    if player.is_null() {
        return Err("could not create a player".into());
    }

    let out = (|| -> Result<String, String> {
        let rc = unsafe { (api.set_fd_source)(player, file.as_raw_fd(), 0, size) };
        if rc != 0 {
            return Err(format!("source: {rc}"));
        }
        let rc = unsafe { (api.set_surface)(player, window) };
        if rc != 0 {
            return Err(format!("surface: {rc}"));
        }
        let rc = unsafe { (api.prepare)(player) };
        if rc != 0 {
            return Err(format!("prepare: {rc}"));
        }
        let rc = unsafe { (api.play)(player) };
        if rc != 0 {
            return Err(format!("play: {rc}"));
        }
        // Let the state settle before reporting it -- prepare/play are async
        // and reading immediately reports whatever it was a moment ago.
        std::thread::sleep(std::time::Duration::from_millis(400));
        let mut st: i32 = -1;
        let mut dur: i32 = 0;
        unsafe {
            (api.get_state)(player, &mut st);
            (api.get_duration)(player, &mut dur);
        }
        Ok(format!(
            "{{\"path\":{},\"bytes\":{},\"state\":{},\"durationMs\":{}}}",
            json_str(path),
            size,
            json_str(state_name(st)),
            dur
        ))
    })();

    match out {
        Ok(j) => {
            if let Ok(mut p) = PLAYING.lock() {
                *p = Some(Playing { player });
            }
            Ok(j)
        }
        Err(e) => {
            unsafe { (api.release)(player) };
            Err(e)
        }
    }
}

pub fn video_stop() -> Result<String, String> {
    let api = player_api().ok_or("video player unavailable on this device")?;
    let Some(p) = PLAYING.lock().ok().and_then(|mut g| g.take()) else {
        return Ok("{\"playing\":false}".into());
    };
    unsafe {
        (api.stop)(p.player);
        (api.release)(p.player);
    }
    Ok("{\"playing\":false}".into())
}
