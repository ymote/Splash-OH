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

// A built-in tool.
call('device.info').then(d => row('device', `${d.productModel} · ${d.osFullName}`))
// A tool from plugin/ — your own Rust, callable by name.
call('app.greet', { name: 'world' }).then(r => row('app.greet', r))
