import './style.css'

// window.splash comes from /__splash.js. Every native call goes through it and
// every one returns a promise.
const call = (tool, args) => window.splash.invoke(tool, args)

const app = document.querySelector('#app')
app.innerHTML = `
  <h1>My Splash App</h1>
  <p class="sub">edit src/main.js — it reloads on the phone</p>
  <ul id="rows"></ul>
`

const row = (label, value) => {
  const li = document.createElement('li')
  li.innerHTML = `<span>${label}</span><b>${value}</b>`
  document.querySelector('#rows').append(li)
}

// A built-in tool. Works as soon as the app starts.
call('device.info')
  .then(d => row('device', `${d.productModel} · ${d.osFullName}`))
  .catch(e => row('device', String(e)))

// A tool from plugin/ — your own Rust.
//
// This one fails until the plugin is linked into the app shell, which is two
// edits in the Splash-OH checkout; see "Add a native capability" in README.md.
// It is here failing rather than absent because a template that only shows
// what already works does not show you where the seam is.
call('app.greet', { name: 'world' })
  .then(r => row('app.greet', r))
  .catch(() => row('app.greet', 'not linked yet — see README'))
