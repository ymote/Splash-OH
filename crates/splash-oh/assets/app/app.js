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

// 7-9. Permission requests. `permission.request` used to forward whatever names
//      a page passed straight to requestPermissionsFromUser, so any trusted
//      page could raise a camera or microphone dialog whenever it liked.
//
//      Both directions are checked here, because a gate proven only in the deny
//      direction is a gate that might refuse everything.
function permCheck(id, names, wantRefused) {
  if (!window.splash || !window.splash.invoke) { mark(id, false, 'no shim'); return; }
  window.splash.invoke('permission.request', names)
    .then(function (r) {
      // For the allow direction, "the promise resolved" is not the pass
      // condition -- that would hold even if the call had been quietly dropped.
      // ArkTS builds {granted, asked} from the OS result, so `asked` counting
      // what was sent is what shows the request actually got there. What the
      // user then taps is not this test's business.
      var reached = !!(r && r.asked === names.length);
      mark(id, wantRefused ? false : reached,
        wantRefused ? 'NOT refused: ' + JSON.stringify(r)
                    : 'asked=' + (r && r.asked) + ' granted=' + ((r && r.granted) || []).length);
    })
    .catch(function (e) {
      var msg = String(e && e.message ? e.message : e);
      mark(id, wantRefused, msg.length > 42 ? msg.slice(0, 42) + '…' : msg);
    });
}

// Not declared by the app and not page-requestable.
permCheck('r-perm-bad', ['ohos.permission.READ_CONTACTS'], true);
// Five at once, over the per-request limit.
permCheck('r-perm-many', ['ohos.permission.CAMERA', 'ohos.permission.MICROPHONE',
  'ohos.permission.LOCATION', 'ohos.permission.APPROXIMATELY_LOCATION',
  'ohos.permission.ACCESS_BLUETOOTH'], true);
// The allow direction. Deferred, because a real prompt covers the page and the
// rows above should be legible in a screenshot before it appears.
setTimeout(function () {
  permCheck('r-perm-ok', ['ohos.permission.LOCATION'], false);
}, 6000);

// 10-12. A tool defined in a crate the bridge does not depend on.
//
//        splash-oh-plugin-demo depends on splash-oh-core and nothing else. It
//        cannot see bridge.rs. If these answer, a capability was added without
//        editing the framework -- which is the thing that separated this from
//        a framework anyone else could build on.
if (window.splash && window.splash.invoke) {
  window.splash.invoke('demo.reverse', 'OpenHarmony')
    .then(function (r) { mark('r-plugin', r === 'ynomraHnepO', String(r)); })
    .catch(function (e) { mark('r-plugin', false, String(e)); });

  window.splash.invoke('demo.sum', { a: 19, b: 23 })
    .then(function (r) { mark('r-plugin2', Number(r) === 42, 'a+b=' + r); })
    .catch(function (e) { mark('r-plugin2', false, String(e)); });

  window.splash.invoke('plugin.list', {})
    .then(function (r) {
      var have = Array.isArray(r) && r.indexOf('demo.sum') >= 0 && r.indexOf('demo.reverse') >= 0;
      mark('r-plugin-list', have, Array.isArray(r) ? r.join(', ') : String(r));
    })
    .catch(function (e) { mark('r-plugin-list', false, String(e)); });
}
