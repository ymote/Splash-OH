// Thin C shim over the ArkUI NDK.
//
// Why a shim rather than binding ArkUI_NativeNodeAPI_1 directly from Rust: that
// struct is a long list of function pointers whose order IS the ABI. Declaring
// it by hand in Rust means silently calling the wrong slot if the order ever
// shifts. Here the real header does the checking, and Rust only ever sees a
// handful of stable, flat `extern "C"` calls.
//
// Nothing in this file touches ArkTS. Every widget is created, configured and
// mounted from native code.

// Built as C++ on purpose. The SDK's ArkUI headers assume a C++ translation
// unit: native_type.h uses bare `bool`, and native_node.h refers to
// `OH_PixelmapNative` without a `struct` tag. Both are hard errors in C.
// Everything exported below is `extern "C"`, so Rust still sees a flat C ABI.
#include <cstdint>
#include <cstring>

#include <arkui/native_node.h>
#include <arkui/native_interface.h>
#include <arkui/ui_input_event.h>
#include <arkui/native_animate.h>
#include <arkui/native_type.h>

#include <network/netstack/net_http.h>
#include <mutex>
#include <condition_variable>
#include <chrono>
#include <string>
#include <cstdlib>

static ArkUI_NativeNodeAPI_1 *g_api = nullptr;

extern "C" {

int splash_arkui_init(void) {
    if (g_api) {
        return 0;
    }
    OH_ArkUI_GetModuleInterface(ARKUI_NATIVE_NODE, ArkUI_NativeNodeAPI_1, g_api);
    return g_api ? 0 : -1;
}

ArkUI_NodeHandle splash_create_node(int type) {
    if (!g_api) return nullptr;
    return g_api->createNode((ArkUI_NodeType)type);
}

void splash_dispose_node(ArkUI_NodeHandle n) {
    if (g_api && n) g_api->disposeNode(n);
}

int splash_add_child(ArkUI_NodeHandle parent, ArkUI_NodeHandle child) {
    if (!g_api || !parent || !child) return -1;
    return g_api->addChild(parent, child);
}

int splash_remove_child(ArkUI_NodeHandle parent, ArkUI_NodeHandle child) {
    if (!g_api || !parent || !child) return -1;
    return g_api->removeChild(parent, child);
}

// ---- attributes -----------------------------------------------------------
// ArkUI attributes are a tagged union: some take a string, some take an array
// of ArkUI_NumberValue (f32/i32/u32). These wrappers cover the shapes the
// Splash backend needs; anything richer can be added as another arm.

int splash_set_string(ArkUI_NodeHandle n, int attr, const char *s) {
    if (!g_api || !n) return -1;
    ArkUI_AttributeItem item;
    std::memset(&item, 0, sizeof(item));
    item.string = s;
    return g_api->setAttribute(n, (ArkUI_NodeAttributeType)attr, &item);
}

// A few attributes are not string-only or number-only but both at once. The
// text picker's range is the example: `.string` carries the items and
// `.value[0].i32` says how to read them. Setting only the string leaves the
// item count at zero, and the picker then draws its rows empty -- which is
// exactly what the catalog's Text picker screen did.
int splash_set_i32_string(ArkUI_NodeHandle n, int attr, int v, const char *s) {
    if (!g_api || !n) return -1;
    ArkUI_NumberValue nv[1];
    nv[0].i32 = v;
    ArkUI_AttributeItem item;
    std::memset(&item, 0, sizeof(item));
    item.value = nv;
    item.size = 1;
    item.string = s;
    return g_api->setAttribute(n, (ArkUI_NodeAttributeType)attr, &item);
}

int splash_set_f32(ArkUI_NodeHandle n, int attr, float v) {
    if (!g_api || !n) return -1;
    ArkUI_NumberValue nv[1];
    nv[0].f32 = v;
    ArkUI_AttributeItem item;
    std::memset(&item, 0, sizeof(item));
    item.value = nv;
    item.size = 1;
    return g_api->setAttribute(n, (ArkUI_NodeAttributeType)attr, &item);
}

// Read one f32 out of an attribute. 0 on success.
//
// The only getter here, and it exists for one reason: a tap rebuilds the whole
// tree, ArkUI's Scroll is a new node each time, and the view snapped back to
// the top. Checking a checkbox halfway down a screen left you at the top of it.
int splash_get_f32(ArkUI_NodeHandle n, int attr, int index, float *out) {
    if (!g_api || !n || !out || index < 0) return -1;
    const ArkUI_AttributeItem *item =
        g_api->getAttribute(n, (ArkUI_NodeAttributeType)attr);
    if (!item || !item->value || item->size <= index) return -1;
    *out = item->value[index].f32;
    return 0;
}

// Read a string attribute into the caller's buffer. Returns the length, or -1.
//
// The counterpart of splash_get_f32, and it exists for the search field: the
// text lives on the node, and an on-change event only says that it moved.
int splash_get_string(ArkUI_NodeHandle n, int attr, char *out, int cap) {
    if (!g_api || !n || !out || cap <= 0) return -1;
    const ArkUI_AttributeItem *item =
        g_api->getAttribute(n, (ArkUI_NodeAttributeType)attr);
    if (!item || !item->string) return -1;
    int len = (int)std::strlen(item->string);
    if (len > cap - 1) len = cap - 1;
    std::memcpy(out, item->string, (size_t)len);
    out[len] = 0;
    return len;
}

int splash_set_i32(ArkUI_NodeHandle n, int attr, int32_t v) {
    if (!g_api || !n) return -1;
    ArkUI_NumberValue nv[1];
    nv[0].i32 = v;
    ArkUI_AttributeItem item;
    std::memset(&item, 0, sizeof(item));
    item.value = nv;
    item.size = 1;
    return g_api->setAttribute(n, (ArkUI_NodeAttributeType)attr, &item);
}

int splash_set_u32(ArkUI_NodeHandle n, int attr, uint32_t v) {
    if (!g_api || !n) return -1;
    ArkUI_NumberValue nv[1];
    nv[0].u32 = v;
    ArkUI_AttributeItem item;
    std::memset(&item, 0, sizeof(item));
    item.value = nv;
    item.size = 1;
    return g_api->setAttribute(n, (ArkUI_NodeAttributeType)attr, &item);
}

// f32 vector — padding (4), border radius (4), translate (3), etc.
int splash_set_f32v(ArkUI_NodeHandle n, int attr, const float *v, int count) {
    if (!g_api || !n || count <= 0 || count > 8) return -1;
    ArkUI_NumberValue nv[8];
    for (int i = 0; i < count; i++) nv[i].f32 = v[i];
    ArkUI_AttributeItem item;
    std::memset(&item, 0, sizeof(item));
    item.value = nv;
    item.size = count;
    return g_api->setAttribute(n, (ArkUI_NodeAttributeType)attr, &item);
}

// A linear gradient. The only attribute here whose payload is a struct rather
// than numbers, so it cannot go through splash_set_f32v.
//
// `dir` is ArkUI_LinearGradientDirection (3 = BOTTOM, i.e. top to bottom).
// ArkUI copies the stops during setAttribute, so the locals are safe to drop.
int splash_set_gradient(ArkUI_NodeHandle n, int attr, int dir,
                        const uint32_t *colors, const float *stops, int count) {
    if (!g_api || !n || !colors || !stops || count <= 0 || count > 16) return -1;
    uint32_t c[16];
    float s[16];
    for (int i = 0; i < count; i++) { c[i] = colors[i]; s[i] = stops[i]; }
    ArkUI_ColorStop cs;
    cs.colors = c;
    cs.stops = s;
    cs.size = count;
    ArkUI_NumberValue nv[3];
    nv[0].f32 = 180.f;
    nv[1].i32 = dir;
    nv[2].i32 = 0;
    ArkUI_AttributeItem item;
    std::memset(&item, 0, sizeof(item));
    item.value = nv;
    item.size = 3;
    item.object = &cs;
    return g_api->setAttribute(n, (ArkUI_NodeAttributeType)attr, &item);
}

// ---- animation ------------------------------------------------------------
// ArkUI animates a *state change*: you hand animateTo a closure, set attributes
// inside it, and the system tweens whatever moved. The closure has to run on
// the C side, so Rust passes a plain function pointer and an opaque argument.
//
// The UI context comes from any mounted node, so no ArkTS is involved here
// either.

static ArkUI_NativeAnimateAPI_1 *g_anim = nullptr;

// The caller's closure owns memory on the Rust side and must be run exactly
// once, whatever happens here. Every early return therefore runs it, and the
// animateTo path runs it afterwards if ArkUI did not -- otherwise a rejected
// animateTo would both leak the closure and silently skip the state change it
// was carrying.
struct SplashAnimCall {
    void (*update)(void *);
    void *user;
    bool ran;
};

static void splash_anim_thunk(void *p) {
    SplashAnimCall *c = (SplashAnimCall *)p;
    if (!c || c->ran) return;
    c->ran = true;
    c->update(c->user);
}

int splash_animate(ArkUI_NodeHandle anchor, int duration_ms, int curve,
                   void (*update)(void *), void *user) {
    if (!update) return -1;
    SplashAnimCall call;
    call.update = update;
    call.user = user;
    call.ran = false;

    if (!g_anim) {
        OH_ArkUI_GetModuleInterface(ARKUI_NATIVE_ANIMATE, ArkUI_NativeAnimateAPI_1, g_anim);
    }
    ArkUI_ContextHandle ctx = anchor ? OH_ArkUI_GetContextByNode(anchor) : nullptr;
    ArkUI_AnimateOption *opt = (g_anim && ctx) ? OH_ArkUI_AnimateOption_Create() : nullptr;
    if (!opt) {
        // No animation available: still apply the change, just without the
        // tween. A missing animation must never mean a missing update.
        splash_anim_thunk(&call);
        return -1;
    }
    OH_ArkUI_AnimateOption_SetDuration(opt, duration_ms);
    OH_ArkUI_AnimateOption_SetCurve(opt, (ArkUI_AnimationCurve)curve);
    ArkUI_ContextCallback cb;
    cb.userData = &call;
    cb.callback = splash_anim_thunk;
    int rc = g_anim->animateTo(ctx, opt, &cb, nullptr);
    OH_ArkUI_AnimateOption_Dispose(opt);
    // animateTo runs the closure synchronously; if it declined to, run it here
    // so the update still lands and the box is still reclaimed.
    splash_anim_thunk(&call);
    return rc;
}

// Where a mounted node actually ended up, in px.
//
// The content area is not the display: the system keeps a strip at the top for
// the status bar and one at the bottom for the navigation gesture. Those differ
// per device, so asking the node beats subtracting constants measured on one
// phone.
int splash_layout_size(ArkUI_NodeHandle n, int32_t *w, int32_t *h) {
    if (!n || !w || !h) return -1;
    ArkUI_IntSize size = {0, 0};
    int rc = OH_ArkUI_NodeUtils_GetLayoutSize(n, &size);
    if (rc != 0) return rc;
    *w = size.width;
    *h = size.height;
    return 0;
}

// ---- mounting into the page ----------------------------------------------
// The ONE place ArkTS is involved: it hands us a NodeContent slot once, at
// startup. After that the whole tree lives here.

int splash_content_add(ArkUI_NodeContentHandle content, ArkUI_NodeHandle root) {
    if (!content || !root) return -1;
    return OH_ArkUI_NodeContent_AddNode(content, root);
}

int splash_content_remove(ArkUI_NodeContentHandle content, ArkUI_NodeHandle root) {
    if (!content || !root) return -1;
    return OH_ArkUI_NodeContent_RemoveNode(content, root);
}

// ---- events ---------------------------------------------------------------

int splash_register_event(ArkUI_NodeHandle n, int event_type, int32_t id) {
    if (!g_api || !n) return -1;
    return g_api->registerNodeEvent(n, (ArkUI_NodeEventType)event_type, id, NULL);
}

// Rust installs one handler; the C side unpacks the event and forwards the
// target id, so Rust never has to know ArkUI_NodeEvent's layout.
static void (*g_rust_handler)(int32_t target_id, int32_t event_type) = nullptr;

// Only the target id and the event type cross into Rust, so ArkUI_NodeEvent's
// layout never leaks across the boundary.
//
// Nothing is read out of the event payload. A scroll handler that accumulated
// the per-event deltas here drifted badly against the real position; the Scroll
// node reports its own offset through NODE_SCROLL_OFFSET, which is what callers
// use instead.
// Swipes.
//
// Wonderous pages its home screen, its photo wall and its artifact carousel by
// swiping; none of that is a component event ArkUI will report. What it will
// report is the raw touch stream, so the drag is measured here and only the
// result -- one of four directions -- crosses into Rust, as an id.
//
// The threshold is the app's own: EightWaySwipeDetector uses 30.
#define SPLASH_SWIPE_THRESHOLD 30.f
static float g_touch_x0 = 0.f, g_touch_y0 = 0.f;
static int32_t g_touch_target = 0;
static bool g_touch_down = false;

// A swipe on target `id` is reported as `id + 1..4` for left, right, up, down.
// The caller reserves that block when it registers the touch event.
enum { SPLASH_SWIPE_LEFT = 1, SPLASH_SWIPE_RIGHT, SPLASH_SWIPE_UP, SPLASH_SWIPE_DOWN };

static void splash_handle_touch(ArkUI_NodeEvent *e, int32_t target) {
    ArkUI_UIInputEvent *in = OH_ArkUI_NodeEvent_GetInputEvent(e);
    if (!in) return;
    const int32_t action = OH_ArkUI_UIInputEvent_GetAction(in);
    const float x = OH_ArkUI_PointerEvent_GetX(in);
    const float y = OH_ArkUI_PointerEvent_GetY(in);
    if (action == UI_TOUCH_EVENT_ACTION_DOWN) {
        g_touch_x0 = x;
        g_touch_y0 = y;
        g_touch_target = target;
        g_touch_down = true;
        return;
    }
    if (action != UI_TOUCH_EVENT_ACTION_UP || !g_touch_down ||
        g_touch_target != target) {
        return;
    }
    g_touch_down = false;
    const float dx = x - g_touch_x0, dy = y - g_touch_y0;
    const float adx = dx < 0 ? -dx : dx, ady = dy < 0 ? -dy : dy;
    // Eight-way, like EightWaySwipeDetector: each axis that clears the
    // threshold is reported, so a diagonal is a horizontal move and a vertical
    // one. Taking only the dominant axis turned every diagonal into half a
    // move, which on the photo wall is a visible difference from the app.
    if (adx >= SPLASH_SWIPE_THRESHOLD) {
        g_rust_handler(target + (dx < 0 ? SPLASH_SWIPE_LEFT : SPLASH_SWIPE_RIGHT),
                       (int32_t)NODE_TOUCH_EVENT);
    }
    if (ady >= SPLASH_SWIPE_THRESHOLD) {
        g_rust_handler(target + (dy < 0 ? SPLASH_SWIPE_UP : SPLASH_SWIPE_DOWN),
                       (int32_t)NODE_TOUCH_EVENT);
    }
}

static void splash_event_trampoline(ArkUI_NodeEvent *e) {
    if (!e || !g_rust_handler) return;
    const int32_t target = OH_ArkUI_NodeEvent_GetTargetId(e);
    const ArkUI_NodeEventType ty = OH_ArkUI_NodeEvent_GetEventType(e);
    if (ty == NODE_TOUCH_EVENT) {
        splash_handle_touch(e, target);
        return;
    }
    g_rust_handler(target, (int32_t)ty);
}

void splash_set_event_handler(void (*h)(int32_t, int32_t)) {
    g_rust_handler = h;
    if (g_api) g_api->registerNodeEventReceiver(splash_event_trampoline);
}

int32_t splash_event_target_id(ArkUI_NodeEvent *e) {
    return e ? OH_ArkUI_NodeEvent_GetTargetId(e) : -1;
}

int32_t splash_event_type(ArkUI_NodeEvent *e) {
    return e ? (int32_t)OH_ArkUI_NodeEvent_GetEventType(e) : -1;
}

// ---- HTTP: a blocking GET over OpenHarmony's native netstack ----------------
// The Splash VM's `http_get` / `fetch_*` capabilities call this so the DSL can
// pull live data itself — nothing baked. OH_Http_Request is asynchronous and
// its response callback is a bare C function pointer with no user-data slot, so
// calls are serialised with a mutex and the single in-flight result is handed
// back through file-static state. netstack delivers the callback on its own
// worker thread, so blocking the caller here does not deadlock.
static std::mutex              g_http_serialize;
static std::mutex              g_http_mu;
static std::condition_variable g_http_cv;
static std::string             g_http_body;
static int                     g_http_code;
static bool                    g_http_done;

static void splash_http_cb(Http_Response *resp, uint32_t errCode) {
    std::lock_guard<std::mutex> lk(g_http_mu);
    g_http_body.clear();
    if (errCode == 0 && resp) {
        g_http_code = (int)resp->responseCode;
        if (resp->body.buffer && resp->body.length) {
            g_http_body.assign(resp->body.buffer, resp->body.length);
        }
    } else {
        g_http_code = -(int)errCode;
    }
    g_http_done = true;
    g_http_cv.notify_one();
}

// Blocking GET. Returns the HTTP status code (negative on transport error).
// If out_buf != null, the body is malloc'd into *out_buf / *out_len and the
// caller must release it with splash_free.
int splash_http_get(const char *url, char **out_buf, int *out_len) {
    std::lock_guard<std::mutex> serialize(g_http_serialize);
    if (out_buf) *out_buf = nullptr;
    if (out_len) *out_len = 0;

    Http_Request *req = OH_Http_CreateRequest(url);
    if (!req) return -1000;

    {
        std::lock_guard<std::mutex> lk(g_http_mu);
        g_http_done = false;
        g_http_code = 0;
        g_http_body.clear();
    }

    Http_EventsHandler handler;
    memset(&handler, 0, sizeof(handler));
    int rc = OH_Http_Request(req, splash_http_cb, handler);
    if (rc != 0) {
        OH_Http_Destroy(&req);
        return -2000 - rc;
    }

    std::unique_lock<std::mutex> lk(g_http_mu);
    g_http_cv.wait_for(lk, std::chrono::seconds(25), [] { return g_http_done; });
    if (!g_http_done) {
        lk.unlock();
        OH_Http_Destroy(&req);
        return -3000; // timed out
    }
    int code = g_http_code;
    if (out_buf && out_len && !g_http_body.empty()) {
        int n = (int)g_http_body.size();
        char *buf = (char *)malloc((size_t)n);
        if (buf) {
            memcpy(buf, g_http_body.data(), (size_t)n);
            *out_buf = buf;
            *out_len = n;
        }
    }
    lk.unlock();
    OH_Http_Destroy(&req);
    return code;
}

void splash_free(char *p) { free(p); }

} // extern "C"

// ---- constants, computed by the compiler -----------------------------------
// ArkUI's enums are mostly implicit (`NODE_HEIGHT,` with no `= N`) and the
// per-component blocks use `1000 * ARKUI_NODE_X + n`. Transcribing them by hand
// — or parsing the header — gets values wrong in ways that only show up as a
// SIGSEGV at runtime. Let the compiler evaluate them and export the results.
#define SPLASH_CONST(name, expr) extern "C" const int32_t name = (int32_t)(expr);

SPLASH_CONST(splash_a_width,        NODE_WIDTH)
SPLASH_CONST(splash_a_height,       NODE_HEIGHT)
SPLASH_CONST(splash_a_bg,           NODE_BACKGROUND_COLOR)
SPLASH_CONST(splash_a_padding,      NODE_PADDING)
SPLASH_CONST(splash_a_margin,       NODE_MARGIN)
SPLASH_CONST(splash_a_border_width, NODE_BORDER_WIDTH)
SPLASH_CONST(splash_a_border_radius,NODE_BORDER_RADIUS)
SPLASH_CONST(splash_a_blur,         NODE_BLUR)
SPLASH_CONST(splash_a_border_color, NODE_BORDER_COLOR)
SPLASH_CONST(splash_a_alignment,    NODE_ALIGNMENT)
SPLASH_CONST(splash_a_opacity,      NODE_OPACITY)
SPLASH_CONST(splash_a_visibility,   NODE_VISIBILITY)
SPLASH_CONST(splash_a_text_content, NODE_TEXT_CONTENT)
SPLASH_CONST(splash_a_font_color,   NODE_FONT_COLOR)
SPLASH_CONST(splash_a_font_size,    NODE_FONT_SIZE)
SPLASH_CONST(splash_a_font_weight,  NODE_FONT_WEIGHT)
SPLASH_CONST(splash_a_text_align,   NODE_TEXT_ALIGN)
SPLASH_CONST(splash_a_button_label, NODE_BUTTON_LABEL)
SPLASH_CONST(splash_a_progress_value, NODE_PROGRESS_VALUE)
SPLASH_CONST(splash_a_progress_total, NODE_PROGRESS_TOTAL)
SPLASH_CONST(splash_a_input_placeholder, NODE_TEXT_INPUT_PLACEHOLDER)
SPLASH_CONST(splash_a_input_text,        NODE_TEXT_INPUT_TEXT)
SPLASH_CONST(splash_a_image_src,   NODE_IMAGE_SRC)
SPLASH_CONST(splash_a_image_fit,   NODE_IMAGE_OBJECT_FIT)
SPLASH_CONST(splash_a_checkbox_select,   NODE_CHECKBOX_SELECT)
SPLASH_CONST(splash_a_checkbox_color,    NODE_CHECKBOX_SELECT_COLOR)
SPLASH_CONST(splash_a_radio_checked,     NODE_RADIO_CHECKED)
SPLASH_CONST(splash_a_toggle_value,      NODE_TOGGLE_VALUE)
SPLASH_CONST(splash_a_toggle_color,      NODE_TOGGLE_SELECTED_COLOR)
SPLASH_CONST(splash_a_textpicker_range,  NODE_TEXT_PICKER_OPTION_RANGE)
// Absolute placement and compositing, for illustrations built from layers.
// A Stack centres its children; Wonderous positions each piece of a wonder
// against the frame, so the pieces need coordinates of their own.
SPLASH_CONST(splash_a_position,     NODE_POSITION)
SPLASH_CONST(splash_a_hit_test,     NODE_HIT_TEST_BEHAVIOR)
SPLASH_CONST(splash_e_did_scroll,   NODE_SCROLL_EVENT_ON_DID_SCROLL)
SPLASH_CONST(splash_a_text_shadow,  NODE_TEXT_TEXT_SHADOW)
SPLASH_CONST(splash_a_translate,    NODE_TRANSLATE)
SPLASH_CONST(splash_a_scale,        NODE_SCALE)
SPLASH_CONST(splash_a_zindex,       NODE_Z_INDEX)
SPLASH_CONST(splash_a_clip,         NODE_CLIP)
SPLASH_CONST(splash_a_stack_align,  NODE_STACK_ALIGN_CONTENT)
SPLASH_CONST(splash_a_linear_gradient, NODE_LINEAR_GRADIENT)

// Added for the flutter/samples kit, which is authored against the makepad
// backend's richer attribute set. Percent sizing is how that kit's `fillw`/
// `fillh` (makepad's Fill) express themselves in ArkUI; the layout attributes
// carry its `alignx`/`aligny`, and FONT_FAMILY carries `icon`.
SPLASH_CONST(splash_a_width_percent,  NODE_WIDTH_PERCENT)
SPLASH_CONST(splash_a_height_percent, NODE_HEIGHT_PERCENT)
SPLASH_CONST(splash_a_font_family,    NODE_FONT_FAMILY)
SPLASH_CONST(splash_a_row_align,      NODE_ROW_ALIGN_ITEMS)
SPLASH_CONST(splash_a_row_justify,    NODE_ROW_JUSTIFY_CONTENT)
SPLASH_CONST(splash_a_col_align,      NODE_COLUMN_ALIGN_ITEMS)
SPLASH_CONST(splash_a_col_justify,    NODE_COLUMN_JUSTIFY_CONTENT)
SPLASH_CONST(splash_a_shadow,         NODE_SHADOW)
SPLASH_CONST(splash_a_layout_weight,  NODE_LAYOUT_WEIGHT)
SPLASH_CONST(splash_a_slider_value,   NODE_SLIDER_VALUE)
SPLASH_CONST(splash_a_slider_min,     NODE_SLIDER_MIN_VALUE)
SPLASH_CONST(splash_a_slider_max,     NODE_SLIDER_MAX_VALUE)
SPLASH_CONST(splash_a_slider_selected, NODE_SLIDER_SELECTED_COLOR)
SPLASH_CONST(splash_a_slider_block,    NODE_SLIDER_BLOCK_COLOR)
SPLASH_CONST(splash_a_slider_track,    NODE_SLIDER_TRACK_COLOR)
SPLASH_CONST(splash_a_checkbox_shape, NODE_CHECKBOX_SHAPE)
SPLASH_CONST(splash_a_loading_color,  NODE_LOADING_PROGRESS_COLOR)
SPLASH_CONST(splash_a_progress_color, NODE_PROGRESS_COLOR)
SPLASH_CONST(splash_a_scroll_offset,  NODE_SCROLL_OFFSET)

SPLASH_CONST(splash_t_text,     ARKUI_NODE_TEXT)
SPLASH_CONST(splash_t_image,    ARKUI_NODE_IMAGE)
SPLASH_CONST(splash_t_toggle,   ARKUI_NODE_TOGGLE)
SPLASH_CONST(splash_t_loading,  ARKUI_NODE_LOADING_PROGRESS)
SPLASH_CONST(splash_t_input,    ARKUI_NODE_TEXT_INPUT)
SPLASH_CONST(splash_t_textarea, ARKUI_NODE_TEXT_AREA)
SPLASH_CONST(splash_t_button,   ARKUI_NODE_BUTTON)
SPLASH_CONST(splash_t_progress, ARKUI_NODE_PROGRESS)
SPLASH_CONST(splash_t_checkbox, ARKUI_NODE_CHECKBOX)
SPLASH_CONST(splash_t_datepicker, ARKUI_NODE_DATE_PICKER)
SPLASH_CONST(splash_t_slider,   ARKUI_NODE_SLIDER)
SPLASH_CONST(splash_t_radio,    ARKUI_NODE_RADIO)
SPLASH_CONST(splash_t_stack,    ARKUI_NODE_STACK)
SPLASH_CONST(splash_t_scroll,   ARKUI_NODE_SCROLL)
SPLASH_CONST(splash_t_column,   ARKUI_NODE_COLUMN)
SPLASH_CONST(splash_t_row,      ARKUI_NODE_ROW)
SPLASH_CONST(splash_t_flex,     ARKUI_NODE_FLEX)
SPLASH_CONST(splash_t_timepicker, ARKUI_NODE_TIME_PICKER)
SPLASH_CONST(splash_t_textpicker, ARKUI_NODE_TEXT_PICKER)
SPLASH_CONST(splash_t_swiper,   ARKUI_NODE_SWIPER)
SPLASH_CONST(splash_t_grid,     ARKUI_NODE_GRID)
SPLASH_CONST(splash_t_waterflow, ARKUI_NODE_WATER_FLOW)
SPLASH_CONST(splash_t_refresh,  ARKUI_NODE_REFRESH)
SPLASH_CONST(splash_t_list,     ARKUI_NODE_LIST)
SPLASH_CONST(splash_e_click,    NODE_ON_CLICK)
SPLASH_CONST(splash_e_touch,    NODE_TOUCH_EVENT)
SPLASH_CONST(splash_e_appear,   NODE_EVENT_ON_APPEAR)
SPLASH_CONST(splash_e_input_change, NODE_TEXT_INPUT_ON_CHANGE)
SPLASH_CONST(splash_e_slider_change, NODE_SLIDER_EVENT_ON_CHANGE)
