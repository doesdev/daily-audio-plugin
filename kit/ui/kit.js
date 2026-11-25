// The editor's side of every plugin: the bridge to the host, a store of
// what the host last said, and the controls a page is composed from.
// Plain ES modules, no framework and no build step.
//
// A plugin's main.js composes its page:
//
//   import { mount, group, row, knob, meter } from './kit/kit.js'
//   mount(({ p, m }) => [
//     group('Drive', row(knob(p.drive), knob(p.tone))),
//     meter(m.out)
//   ])
//
// `p` holds a handle for each setting by its key and `m` one for each
// meter. A page that only wants the settings laid out by their groups
// is `mount(auto)`.

export const VERSION = 1

// ---- values: pure, so they run under node as well

export const clamp = (v, lo = 0, hi = 1) => Math.min(hi, Math.max(lo, v))

// A normalized value snapped to the steps a setting has, if any.
export const snap = (row, norm) => {
  const v = clamp(norm)
  return row.steps > 0 ? Math.round(v * row.steps) / row.steps : v
}

// Where a drag of `dy` pixels up from `from` lands. A fine drag (shift)
// moves a tenth as far; a stepped setting moves a step per 24 pixels at
// most, so a list of two does not flip on a twitch.
export const dragged = (row, from, dy, fine = false) => {
  const span = row.steps > 0 ? Math.max(120, row.steps * 24) : 160
  return clamp(from + dy / (fine ? span * 10 : span))
}

// One notch of the wheel or an arrow key: a step, or a fiftieth.
export const nudged = (row, from, by, fine = false) => {
  const unit = row.steps > 0 ? 1 / row.steps : fine ? 0.002 : 0.02
  return snap(row, from + by * unit)
}

// The sweep of a knob's arc for a value, in degrees from straight up.
export const SWEEP = 270
export const angle = (norm) => (clamp(norm) - 0.5) * SWEEP

// An SVG arc on a circle of radius `r` about (cx, cy), between two
// angles in degrees from straight up, clockwise.
export const arc = (cx, cy, r, from, to) => {
  const at = (deg) => {
    const rad = (deg - 90) * Math.PI / 180
    return [cx + r * Math.cos(rad), cy + r * Math.sin(rad)]
  }
  const [x0, y0] = at(from)
  const [x1, y1] = at(to)
  const large = Math.abs(to - from) > 180 ? 1 : 0
  const fix = (n) => n.toFixed(2)
  return `M ${fix(x0)} ${fix(y0)} A ${r} ${r} 0 ${large} 1 ${fix(x1)} ${fix(y1)}`
}

// The text for a value the host has not yet described, as near as the
// page can tell; the host's own text replaces it within a tick.
export const describe = (row, norm) => {
  if (row.kind === 'list') {
    return row.choices[Math.round(clamp(norm) * (row.choices.length - 1))]
  }
  if (row.kind !== 'range') return ''
  const plain = row.lo + Math.pow(clamp(norm), row.skew) * (row.hi - row.lo)
  const stepped = row.step > 0
    ? row.lo + Math.round((plain - row.lo) / row.step) * row.step
    : plain
  const text = stepped.toFixed(row.decimals)
  return row.unit ? `${text} ${row.unit}` : text
}

// ---- the store: what the host last said, folded message by message

export const EMPTY = {
  ready: false,
  name: '',
  build: '',
  params: [],
  meters: [],
  values: [],
  texts: [],
  levels: [],
  levelTexts: []
}

export const reduce = (state, msg) => {
  if (msg.type === 'hello') {
    const { type, version, ...rest } = msg
    return { ...EMPTY, ...rest, ready: true }
  }
  if (msg.type === 'state') {
    return { ...state, values: msg.values, texts: msg.texts }
  }
  if (msg.type === 'meters') {
    return { ...state, levels: msg.levels, levelTexts: msg.levelTexts }
  }
  // The page's own move, shown at once in the page's own words; the
  // host's text follows within a tick.
  if (msg.type === 'local') {
    const { index, value } = msg
    const text = describe(state.params[index], value)
    return {
      ...state,
      values: state.values.map((v, i) => (i === index ? value : v)),
      texts: state.texts.map((t, i) => (i === index ? text : t))
    }
  }
  return state
}

// ---- the bridge

const post = (msg) => window.ipc?.postMessage(JSON.stringify(msg))

const makeStore = () => {
  let state = EMPTY
  const listeners = new Set()
  const tell = (prev) => listeners.forEach((fn) => fn(state, prev))
  return {
    get: () => state,
    on: (fn) => { listeners.add(fn); return () => listeners.delete(fn) },
    receive: (msg) => {
      const prev = state
      state = reduce(state, msg)
      if (state !== prev) tell(prev)
    }
  }
}

// A handle on one setting: what a control reads and how it moves it.
const settingHandle = (store, index) => {
  const row = () => store.get().params[index]
  const move = (value) => {
    const norm = snap(row(), value)
    if (norm === store.get().values[index]) return
    store.receive({ type: 'local', index, value: norm })
    post({ type: 'set', id: row().id, value: norm })
  }
  const begin = () => post({ type: 'begin', id: row().id })
  const end = () => post({ type: 'end', id: row().id })
  return {
    index,
    get row () { return row() },
    get value () { return store.get().values[index] },
    get text () { return store.get().texts[index] ?? '' },
    begin,
    move,
    end,
    // A whole gesture at once: a click, a key, a typed value.
    set: (value) => { begin(); move(value); end() },
    on: (fn) => store.on((s, prev) => {
      const moved = s.values[index] !== prev.values[index]
      if (moved || s.texts[index] !== prev.texts[index]) fn()
    })
  }
}

const meterHandle = (store, index) => ({
  index,
  get row () { return store.get().meters[index] },
  get value () { return store.get().levels[index] ?? 0 },
  get text () { return store.get().levelTexts[index] ?? '' },
  on: (fn) => store.on((s, prev) => {
    if (s.levels[index] !== prev.levels[index]) fn()
  })
})

const byKey = (rows, make) => Object.fromEntries(
  rows.map((row, index) => [row.key, make(index)])
)

// Builds the page once the host says hello. `build` gets the handles
// and returns a node or a list of them.
export const mount = (build, root = document.getElementById('app')) => {
  const store = makeStore()
  window.kit = { receive: store.receive, store }
  let built = false
  store.on((state) => {
    if (built || !state.ready) return
    built = true
    document.title = state.name
    const ui = {
      name: state.name,
      build: state.build,
      p: byKey(state.params, (i) => settingHandle(store, i)),
      m: byKey(state.meters, (i) => meterHandle(store, i)),
      store
    }
    root.replaceChildren(...[build(ui)].flat(Infinity).filter(Boolean))
    root.dataset.ready = 'true'
  })
  // Escape hands the keyboard back to the host, so its transport
  // shortcuts work again after a click in the editor.
  window.addEventListener('keydown', (event) => {
    if (event.key === 'Escape') post({ type: 'blur' })
  })
  post({ type: 'ready' })
  return store
}

// ---- the DOM

const SVG = 'http://www.w3.org/2000/svg'

// An element: h('div.name', { attrs }, ...children).
export const h = (tag, attrs = {}, ...children) => {
  const [name, ...classes] = tag.split('.')
  const svg = ['svg', 'path', 'circle', 'line', 'g'].includes(name)
  const el = svg
    ? document.createElementNS(SVG, name)
    : document.createElement(name)
  if (classes.length) el.setAttribute('class', classes.join(' '))
  Object.entries(attrs).forEach(([key, value]) => {
    if (key.startsWith('on')) el.addEventListener(key.slice(2), value)
    else if (value !== false && value != null) el.setAttribute(key, value)
  })
  el.append(...children.flat(Infinity).filter((c) => c != null && c !== false))
  return el
}

// What every control shares: its title and, unless the control shows
// its own value (`opts.value` false), that value as text.
const labelled = (handle, kind, body, opts = {}) => {
  const out = opts.value === false ? null : h('output.kit-value')
  const el = h(`div.kit-control.kit-${kind}`, {
    'data-key': handle.row.key,
    'data-id': handle.row.id,
    title: handle.row.about || null
  }, h('span.kit-title', {}, opts.title ?? handle.row.short), body, out)
  if (!out) return el
  const show = () => { out.textContent = handle.text }
  handle.on(show)
  show()
  return el
}

// Dragging, the wheel, the arrow keys and double-click to reset, on
// any element that stands for a setting's value.
const adjustable = (el, handle) => {
  let from = null
  el.addEventListener('pointerdown', (event) => {
    if (event.button !== 0) return
    from = { y: event.clientY, value: handle.value }
    el.setPointerCapture(event.pointerId)
    handle.begin()
  })
  el.addEventListener('pointermove', (event) => {
    if (!from) return
    const dy = from.y - event.clientY
    handle.move(dragged(handle.row, from.value, dy, event.shiftKey))
  })
  const stop = () => {
    if (!from) return
    from = null
    handle.end()
  }
  el.addEventListener('pointerup', stop)
  el.addEventListener('pointercancel', stop)
  el.addEventListener('dblclick', () => handle.set(handle.row.default))
  el.addEventListener('wheel', (event) => {
    event.preventDefault()
    const by = event.deltaY < 0 ? 1 : -1
    handle.set(nudged(handle.row, handle.value, by, event.shiftKey))
  }, { passive: false })
  el.addEventListener('keydown', (event) => {
    const by = { ArrowUp: 1, ArrowRight: 1, ArrowDown: -1, ArrowLeft: -1 }
    if (!by[event.key]) return
    event.preventDefault()
    handle.set(nudged(handle.row, handle.value, by[event.key], event.shiftKey))
  })
  return el
}

export const knob = (handle, opts = {}) => {
  const track = h('path.kit-knob-track', { d: arc(24, 24, 18, -135, 135) })
  const fill = h('path.kit-knob-fill')
  const tick = h('line.kit-knob-tick', { x1: 24, y1: 24, x2: 24, y2: 9 })
  const dial = h('svg.kit-knob-dial', {
    viewBox: '0 0 48 48',
    tabindex: 0,
    role: 'slider',
    'aria-label': handle.row.title
  }, track, fill, h('circle.kit-knob-cap', { cx: 24, cy: 24, r: 12 }), tick)
  const show = () => {
    const deg = angle(handle.value)
    // From the centre for a setting that rests there, else the start.
    const rest = opts.bipolar ? 0 : -135
    const [a, b] = deg < rest ? [deg, rest] : [rest, deg]
    fill.setAttribute('d', b - a < 0.5 ? '' : arc(24, 24, 18, a, b))
    tick.setAttribute('transform', `rotate(${deg} 24 24)`)
    dial.setAttribute('aria-valuetext', handle.text)
  }
  handle.on(show)
  show()
  return labelled(handle, 'knob', adjustable(dial, handle), opts)
}

export const slider = (handle, opts = {}) => {
  const fill = h('div.kit-slider-fill')
  const bar = h('div.kit-slider-bar', {
    tabindex: 0,
    role: 'slider',
    'aria-label': handle.row.title
  }, fill)
  const show = () => { fill.style.width = `${handle.value * 100}%` }
  handle.on(show)
  show()
  return labelled(handle, 'slider', adjustable(bar, handle), opts)
}

// A list as buttons side by side: for a handful of choices.
export const choice = (handle, opts = {}) => {
  const { choices } = handle.row
  const last = Math.max(1, choices.length - 1)
  const buttons = choices.map((name, i) => h('button.kit-choice', {
    type: 'button',
    onclick: () => handle.set(i / last)
  }, name))
  const show = () => {
    const at = Math.round(handle.value * last)
    buttons.forEach((b, i) => b.setAttribute('aria-pressed', i === at))
  }
  handle.on(show)
  show()
  const body = h('div.kit-choices', { role: 'group' }, buttons)
  return labelled(handle, 'choice', body, { ...opts, value: false })
}

// A list as a menu: for more choices than fit side by side.
export const menu = (handle, opts = {}) => {
  const { choices } = handle.row
  const last = Math.max(1, choices.length - 1)
  const select = h('select.kit-select', {
    'aria-label': handle.row.title,
    onchange: (event) => handle.set(event.target.selectedIndex / last)
  }, choices.map((name) => h('option', {}, name)))
  const show = () => { select.selectedIndex = Math.round(handle.value * last) }
  handle.on(show)
  show()
  return labelled(handle, 'menu', select, { ...opts, value: false })
}

export const toggle = (handle, opts = {}) => {
  const button = h('button.kit-toggle', {
    type: 'button',
    role: 'switch',
    'aria-label': handle.row.title,
    onclick: () => handle.set(handle.value > 0.5 ? 0 : 1)
  }, h('span.kit-toggle-thumb'))
  const show = () => button.setAttribute('aria-checked', handle.value > 0.5)
  handle.on(show)
  show()
  return labelled(handle, 'toggle', button, { ...opts, value: false })
}

export const meter = (handle, opts = {}) => {
  const fill = h('div.kit-meter-fill')
  const out = h('output.kit-value')
  const bar = h('div.kit-meter-bar', {
    role: 'meter',
    'aria-label': handle.row.title
  }, fill)
  const show = () => {
    fill.style.setProperty('--level', handle.value)
    out.textContent = handle.text
  }
  handle.on(show)
  show()
  return h(`div.kit-control.kit-meter${opts.vertical ? '.kit-vertical' : ''}`, {
    'data-key': handle.row.key
  }, h('span.kit-title', {}, opts.title ?? handle.row.short), bar, out)
}

// The control a setting gets when the page does not say: a toggle for
// Off and On, buttons for a few choices, a menu for many, else a knob.
export const control = (handle) => {
  const row = handle.row
  if (row.kind !== 'list') return knob(handle)
  if (row.choices.length === 2 && row.choices[0] === 'Off') return toggle(handle)
  return row.choices.length <= 4 ? choice(handle) : menu(handle)
}

export const row = (...children) => h('div.kit-row', {}, children)

export const column = (...children) => h('div.kit-column', {}, children)

export const group = (title, ...children) => h('section.kit-group', {},
  title ? h('h2.kit-group-title', {}, title) : null,
  children)

export const header = (ui, ...children) => h('header.kit-header', {},
  h('h1.kit-name', {}, ui.name),
  children,
  h('span.kit-build', {}, ui.build))

// Every setting under its group's name, then the meters: a whole page
// from the spec alone.
export const auto = (ui) => {
  const handles = Object.values(ui.p)
  const groups = [...new Set(handles.map((handle) => handle.row.group))]
  const meters = Object.values(ui.m).map((handle) => meter(handle))
  return [
    header(ui),
    h('div.kit-body', {},
      groups.map((name) => group(name, row(
        handles.filter((handle) => handle.row.group === name).map(control)
      ))),
      meters.length ? group('', column(meters)) : null)
  ]
}
