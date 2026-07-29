// The spike's checks. Each one writes pass/fail into the page so the result is
// legible in a screenshot rather than only in a log — a device test whose
// outcome you have to go and grep for is a device test that gets misread.

function mark(id, ok, note) {
  var el = document.querySelector('#' + id + ' span');
  if (!el) return;
  el.textContent = (ok ? 'PASS' : 'FAIL') + (note ? ' — ' + note : '');
  el.className = ok ? 'ok' : 'bad';
}

// 1. Stylesheet. If /app.css never arrived the computed colour is the default.
var probe = getComputedStyle(document.querySelector('.sub')).color;
mark('r-css', probe === 'rgb(0, 128, 96)', probe);

// 2. Image. A 404 for the svg leaves naturalWidth at 0.
var img = document.querySelector('img');
function checkImg() { mark('r-img', img.naturalWidth > 0, img.naturalWidth + 'px'); }
img.complete ? checkImg() : (img.onload = checkImg, img.onerror = checkImg);

// 3. Code-split chunk. This is the one a single-file page cannot test: it is a
//    second document-relative request issued at runtime, not at parse time.
import('/chunk.js')
  .then(function (m) { mark('r-chunk', m.answer() === 42, 'answer=' + m.answer()); })
  .catch(function (e) { mark('r-chunk', false, String(e)); });

// 4. A path with nothing behind it must be a real 404, not a hang. ArkWeb has
//    no default resolver for a custom scheme, so a handler that declines
//    produces a request nobody answers.
var t404 = setTimeout(function () { mark('r-404', false, 'timed out (hang)'); }, 4000);
fetch('/does-not-exist')
  .then(function (r) { clearTimeout(t404); mark('r-404', r.status === 404, 'status=' + r.status); })
  .catch(function (e) { clearTimeout(t404); mark('r-404', false, String(e)); });

// 5. The bridge, from a page Rust did not generate. This is the trust question:
//    the shim arrived as a served file, but splash_native is injected by ArkTS
//    and only for slots Rust marks trusted.
if (!window.splash || !window.splash.invoke) {
  mark('r-bridge', false, 'no shim');
} else {
  window.splash.invoke('device.info', {})
    // productModel, not model. The first version of this check asserted a key
    // device.info does not return, so the call succeeded and the row still read
    // FAIL -- with an empty note, because the missing value was also the note.
    .then(function (d) { mark('r-bridge', !!(d && d.productModel), d && d.productModel); })
    .catch(function (e) { mark('r-bridge', false, String(e)); });
}

// 6. Does the document survive a native rerender?
//
//    A live uptime counter, deliberately NOT sessionStorage: a recreated Web
//    component is a new session, so storage would be cleared alongside the
//    document and "runs === 1" would hold whether or not the page survived --
//    a check that cannot fail is not a check. Uptime can fail. Force rerenders
//    for N seconds, screenshot, and compare what the page claims against the
//    wall clock: still climbing means the document lived, back near zero means
//    it was thrown away and reloaded.
var t0 = Date.now();
setInterval(function () {
  var el = document.querySelector('#r-state span');
  if (el) {
    el.textContent = 'uptime ' + Math.round((Date.now() - t0) / 1000) + 's';
    el.className = 'note';
  }
}, 250);
