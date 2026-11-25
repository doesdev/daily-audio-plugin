// The editor's side of every plugin: the bridge to the host, a store of
// what the host last said, and the controls a page is composed from.
// Plain ES modules, no framework and no build step. docs/design.md says
// why each thing looks and behaves as it does.
//
// A plugin's main.js composes its page:
//
//   import {
//     mount, glass, header, picture, sheet, values, value, foot
//   } from './kit/kit.js'
//   mount((ui) => [
//     glass(header(ui), picture(ui, (pen) => { /* what it is doing */ })),
//     sheet(
//       values(value(ui.p.drive, { ui }), value(ui.p.tone, { ui })),
//       foot(ui))
//   ])
//
// `ui.p` holds a handle for each setting by its key and `ui.m` one for
// each meter. `mount(auto)` lays every setting out with no more said.

export const VERSION = 2

// ---- values: pure, so they run under node as well

export const clamp = (v, lo = 0, hi = 1) => Math.min(hi, Math.max(lo, v))

// A normalized value snapped to the steps a setting has, if any.
export const snap = (row, norm) => {
  const v = clamp(norm)
  return row.steps > 0 ? Math.round(v * row.steps) / row.steps : v
}

// Where a drag of `by` pixels from `from` lands on a control `span`
// pixels long. A fine drag (shift) moves a tenth as far.
export const dragged = (row, from, by, fine = false, span = 160) => {
  const length = row.steps > 0 ? Math.max(span, row.steps * 24) : span
  return clamp(from + by / (fine ? length * 10 : length))
}

// One notch of the wheel or an arrow key: a step, or a fiftieth.
export const nudged = (row, from, by, fine = false) => {
  const unit = row.steps > 0 ? 1 / row.steps : fine ? 0.002 : 0.02
  return snap(row, from + by * unit)
}

// A value's text as the number to draw large and the unit to draw
// small beside it: split at the first space.
export const parts = (text) => {
  const at = text.indexOf(' ')
  return at < 0 ? [text, ''] : [text.slice(0, at), text.slice(at + 1)]
}

// A setting's value in its own terms, as the host would work it out.
export const plainOf = (row, norm) => {
  const v = clamp(norm)
  if (row.kind === 'list') return Math.round(v * (row.choices.length - 1))
  const plain = row.lo + Math.pow(v, row.skew) * (row.hi - row.lo)
  return row.step > 0
    ? row.lo + Math.round((plain - row.lo) / row.step) * row.step
    : plain
}

export const normOf = (row, plain) => {
  if (row.kind === 'list') return plain / Math.max(1, row.choices.length - 1)
  const part = clamp((plain - row.lo) / (row.hi - row.lo))
  return Math.pow(part, 1 / row.skew)
}

// The text for a value the host has not yet described, as near as the
// page can tell; the host's own text replaces it within a tick.
export const describe = (row, norm) => {
  if (row.kind === 'list') return row.choices[plainOf(row, norm)]
  if (row.kind !== 'range') return ''
  const text = plainOf(row, norm).toFixed(row.decimals)
  return row.unit ? `${text} ${row.unit}` : text
}

// The normalized value for text a person typed, or null. A number may
// carry the setting's unit or not, and a k for thousands ("8k", "1.2k
// Hz"); a list is chosen by name, or by the start of one.
export const parseValue = (row, text) => {
  const typed = text.trim().toLowerCase()
  if (!typed) return null
  if (row.kind === 'list') {
    const names = row.choices.map((name) => name.toLowerCase())
    const exact = names.indexOf(typed)
    const at = exact >= 0
      ? exact
      : names.findIndex((name) => name.startsWith(typed))
    return at < 0 ? null : normOf(row, at)
  }
  if (row.kind !== 'range') return null
  const unit = row.unit.toLowerCase()
  const bare = (unit && typed.endsWith(unit)
    ? typed.slice(0, -unit.length)
    : typed).trim().replace(',', '.')
  const match = bare.match(/^([+-]?\d*\.?\d+)\s*(k?)$/)
  if (!match) return null
  const plain = Number(match[1]) * (match[2] ? 1000 : 1)
  return snap(row, normOf(row, clamp(plain, row.lo, row.hi)))
}

// The theme to start in: the one chosen before, else the system's.
export const themeFor = (saved, prefersLight) => {
  if (saved === 'light' || saved === 'dark') return saved
  return prefersLight ? 'light' : 'dark'
}

export const otherTheme = (theme) => (theme === 'light' ? 'dark' : 'light')

// How much to scale a page designed at `base` to fill a window, kept
// in proportion and never under half size.
export const fitScale = ([width, height], [baseW, baseH]) =>
  Math.max(0.5, Math.min(width / baseW, height / baseH))

// ---- history: what the page's own gestures changed, to step back
// through and forward again

export const NO_HISTORY = { past: [], future: [] }

// A finished gesture: `index` went from `before` to `after`.
export const recorded = (history, change) => change.before === change.after
  ? history
  : { past: [...history.past, change].slice(-100), future: [] }

// [the history after stepping back, the change undone] or null.
export const undone = ({ past, future }) => past.length
  ? [{ past: past.slice(0, -1), future: [past.at(-1), ...future] }, past.at(-1)]
  : null

export const redone = ({ past, future }) => future.length
  ? [{ past: [...past, future[0]], future: future.slice(1) }, future[0]]
  : null

// The settings that differ between two sets of values: [index, to].
export const differences = (from, to) => to
  .map((value, index) => [index, value])
  .filter(([index, value]) => from[index] !== value)

// ---- density: how much the page shows at once

export const DENSITIES = ['roomy', 'regular', 'dense']

// The density to start in: the one chosen before, else the plugin's.
export const densityFor = (saved, preferred = 'roomy') =>
  DENSITIES.includes(saved) ? saved : preferred

export const nextDensity = (density) =>
  DENSITIES[(DENSITIES.indexOf(density) + 1) % DENSITIES.length]

// ---- meters: six segments, and a peak that holds for a moment

// A level is a fraction of the 60 dB under full scale. Each zone ends
// at `to` dB; the last is the clip lamp, lit at full scale alone.
export const ZONES = [
  { name: 'ink', to: -24 },
  { name: 'green', to: -12 },
  { name: 'yellow', to: -6 },
  { name: 'orange', to: -3 },
  { name: 'red', to: 0 },
  { name: 'lamp', to: 0 }
]

// How full each of the six segments is, 0 to 1, for a level.
export const zoneFills = (level) => {
  const db = clamp(level) * 60 - 60
  return ZONES.map(({ name, to }, i) => {
    if (name === 'lamp') return db >= 0 ? 1 : 0
    const from = i ? ZONES[i - 1].to : -60
    return clamp((db - from) / (to - from))
  })
}

// The name of the segment a level has reached.
export const zoneOf = (level) => {
  const db = clamp(level) * 60 - 60
  return ZONES.find(({ to }) => db < to)?.name ?? 'red'
}

export const HOLD_MS = 1400

// The held peak after a reading of `level` at time `now`.
export const held = (hold, level, now) =>
  level >= hold.level || now - hold.at > HOLD_MS ? { level, at: now } : hold

// ---- the store: what the host last said, folded message by message

export const EMPTY = {
  ready: false,
  name: '',
  build: '',
  size: [640, 400],
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

// The page's own record of its gestures, with compare: two sets of
// settings, A and B, to flip between.
const makeSession = (store) => {
  let history = NO_HISTORY
  let slot = 'A'
  const kept = { A: null, B: null }
  const listeners = new Set()
  const tell = () => listeners.forEach((fn) => fn())

  // One setting moved as a whole gesture, without recording it.
  const put = (index, value) => {
    const id = store.get().params[index].id
    post({ type: 'begin', id })
    store.receive({ type: 'local', index, value })
    post({ type: 'set', id, value })
    post({ type: 'end', id })
  }
  const step = (result, pick) => {
    if (!result) return
    const [next, change] = result
    history = next
    put(change.index, pick(change))
    tell()
  }
  return {
    on: (fn) => { listeners.add(fn); return () => listeners.delete(fn) },
    record: (change) => { history = recorded(history, change); tell() },
    undo: () => step(undone(history), (change) => change.before),
    redo: () => step(redone(history), (change) => change.after),
    get canUndo () { return history.past.length > 0 },
    get canRedo () { return history.future.length > 0 },
    get slot () { return slot },
    // Keeps what is set now under this letter and goes to the other,
    // which starts as a copy: the first flip changes nothing.
    flip: () => {
      const now = [...store.get().values]
      kept[slot] = now
      slot = slot === 'A' ? 'B' : 'A'
      differences(now, kept[slot] ?? now).forEach(([i, v]) => put(i, v))
      tell()
    }
  }
}

// A handle on one setting: what a control reads and how it moves it.
const settingHandle = (store, session, index) => {
  const row = () => store.get().params[index]
  let before = null
  const move = (value) => {
    const norm = snap(row(), value)
    if (norm === store.get().values[index]) return
    store.receive({ type: 'local', index, value: norm })
    post({ type: 'set', id: row().id, value: norm })
  }
  const begin = () => {
    before = store.get().values[index]
    post({ type: 'begin', id: row().id })
  }
  const end = () => {
    post({ type: 'end', id: row().id })
    session.record({ index, before, after: store.get().values[index] })
  }
  return {
    index,
    get row () { return row() },
    get value () { return store.get().values[index] },
    get text () { return store.get().texts[index] ?? '' },
    // The value in the setting's own terms, for a picture to draw from.
    get plain () { return plainOf(row(), store.get().values[index]) },
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

// ---- the theme: light or dark, chosen once for every editor this
// host shows, since they share the page's storage

const THEME_KEY = 'kit-theme'

const savedTheme = () => {
  try { return window.localStorage.getItem(THEME_KEY) } catch { return null }
}

const currentTheme = () => document.documentElement.dataset.theme

const applyTheme = (theme) => {
  document.documentElement.dataset.theme = theme
}

const startTheme = () => applyTheme(themeFor(
  savedTheme(),
  window.matchMedia('(prefers-color-scheme: light)').matches
))

const DENSITY_KEY = 'kit-density'

const currentDensity = () => document.documentElement.dataset.density

const startDensity = (preferred) => {
  let saved = null
  try { saved = window.localStorage.getItem(DENSITY_KEY) } catch {}
  document.documentElement.dataset.density = densityFor(saved, preferred)
}

const switchDensity = () => {
  const next = nextDensity(currentDensity())
  document.documentElement.dataset.density = next
  try { window.localStorage.setItem(DENSITY_KEY, next) } catch {}
  window.dispatchEvent(new Event('kit-density'))
  return next
}

const switchTheme = () => {
  const next = otherTheme(currentTheme())
  applyTheme(next)
  try { window.localStorage.setItem(THEME_KEY, next) } catch {}
  window.dispatchEvent(new Event('kit-theme'))
  return next
}

// The page is designed at the size the spec gives and scaled whole to
// whatever window the host makes it, so nothing reflows or gets small.
const fitTo = (root, base) => {
  const fit = () => {
    const scale = fitScale([window.innerWidth, window.innerHeight], base)
    root.style.width = `${base[0]}px`
    root.style.height = `${base[1]}px`
    root.style.zoom = scale
  }
  window.addEventListener('resize', fit)
  fit()
}

// Builds the page once the host says hello. `build` gets the handles
// and returns a node or a list of them. `opts.density` is the density
// the plugin starts in until the person picks one.
export const mount = (build, opts = {}) => {
  const root = opts.root ?? document.getElementById('app')
  startTheme()
  startDensity(opts.density)
  let touched = null
  const store = makeStore()
  const session = makeSession(store)
  window.kit = {
    receive: store.receive, store, session, touched: () => touched
  }
  let built = false
  store.on((state) => {
    if (built || !state.ready) return
    built = true
    document.title = state.name
    const ui = {
      name: state.name,
      build: state.build,
      p: byKey(state.params, (i) => settingHandle(store, session, i)),
      m: byKey(state.meters, (i) => meterHandle(store, i)),
      store,
      session,
      // What the foot of the sheet says about the control under the
      // pointer; `foot` sets it up.
      hint: () => {},
      // The key of the setting being dragged now, for the picture to
      // show in purple; a control says when it is taken and let go.
      touched: () => touched,
      touch: (key) => {
        touched = key
        window.dispatchEvent(new Event('kit-touch'))
      }
    }
    root.replaceChildren(...[build(ui)].flat(Infinity).filter(Boolean))
    fitTo(root, state.size)
    root.dataset.ready = 'true'
  })
  window.addEventListener('keydown', (event) => {
    const typing = event.target.matches?.('input, select')
    const mod = event.ctrlKey || event.metaKey
    // Escape hands the keyboard back to the host, so its transport
    // shortcuts work again after a click in the editor.
    if (event.key === 'Escape' && !typing) post({ type: 'blur' })
    if (!mod || typing || event.key.toLowerCase() !== 'z') return
    event.preventDefault()
    if (event.shiftKey) session.redo()
    else session.undo()
  })
  post({ type: 'ready' })
  return store
}

// ---- the DOM

const SVG = 'http://www.w3.org/2000/svg'
const SVG_TAGS = ['svg', 'path', 'circle', 'line', 'g', 'rect']

// An element: h('div.name', { attrs }, ...children).
export const h = (tag, attrs = {}, ...children) => {
  const [name, ...classes] = tag.split('.')
  const el = SVG_TAGS.includes(name)
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

// The icon set: one 1.6 line on a 20 grid, round ends, no fills.
export const ICONS = {
  undo: 'M8 5 4.5 8.5 8 12M4.5 8.5H12a3.5 3.5 0 0 1 0 7H9.5',
  redo: 'M12 5l3.5 3.5L12 12M15.5 8.5H8a3.5 3.5 0 0 0 0 7h2.5',
  roomy: 'M4.5 4.5h11v4.5h-11zM4.5 11h11v4.5h-11z',
  regular: 'M4.5 5.5h11M4.5 10h11M4.5 14.5h11',
  dense: 'M4.5 4.5h11M4.5 8.2h11M4.5 11.8h11M4.5 15.5h11',
  theme: 'M10 4a6 6 0 1 0 0 12a6 6 0 1 0 0-12M10 4v12'
}

export const icon = (name) => h('svg.kit-icon', {
  viewBox: '0 0 20 20', 'aria-hidden': 'true'
}, h('path', { d: ICONS[name] }))

// The page's own hint for a control: what it is and what it does.
const hinted = (el, ui, handle) => {
  if (!ui) return el
  const say = () => ui.hint(handle.row.title, handle.row.about)
  el.addEventListener('pointerenter', say)
  el.addEventListener('focusin', say)
  el.addEventListener('pointerleave', () => ui.hint())
  el.addEventListener('focusout', () => ui.hint())
  return el
}

// A value as its number drawn large and its unit small, which can be
// typed over: double-click it, or press Enter on its control. Enter
// keeps what was typed if the setting can hold it; Escape, or anything
// it cannot hold, leaves it as it was.
const valueText = (handle) => {
  const out = h('output.kit-value', { title: 'Double-click to type a value' })
  const show = () => {
    if (out.dataset.editing) return
    const [number, unit] = parts(handle.text)
    out.replaceChildren(
      h('span.kit-number', {}, number),
      unit ? h('span.kit-unit', {}, ` ${unit}`) : ''
    )
  }
  const edit = () => {
    if (out.dataset.editing) return
    out.dataset.editing = 'true'
    const input = h('input.kit-entry', {
      type: 'text', value: handle.text, 'aria-label': handle.row.title
    })
    const done = (keep) => {
      if (!out.dataset.editing) return
      delete out.dataset.editing
      const norm = keep ? parseValue(handle.row, input.value) : null
      if (norm !== null) handle.set(norm)
      show()
    }
    input.addEventListener('keydown', (event) => {
      if (event.key === 'Enter') done(true)
      if (event.key === 'Escape') done(false)
      event.stopPropagation()
    })
    input.addEventListener('pointerdown', (event) => event.stopPropagation())
    input.addEventListener('blur', () => done(true))
    out.replaceChildren(input)
    input.select()
  }
  out.addEventListener('dblclick', (event) => {
    event.stopPropagation()
    edit()
  })
  handle.on(show)
  show()
  return Object.assign(out, { edit })
}

// A control's shell: its key, for the page and its tests to find it by.
const shell = (handle, kind, opts, ...children) => hinted(
  h(`div.kit-control.kit-${kind}`, {
    'data-key': handle.row.key, 'data-id': handle.row.id
  }, children),
  opts.ui, handle
)

// Dragging along it, the wheel, the arrow keys and double-click for
// the default, on the element that stands for a setting's value.
// `mark` is the control that wears the purple while it is held.
const adjustable = (el, handle, mark, ui) => {
  let from = null
  const hold = (on) => {
    if (on) mark.dataset.held = 'true'
    else delete mark.dataset.held
    ui?.touch(on ? handle.row.key : null)
  }
  el.addEventListener('pointerdown', (event) => {
    if (event.button !== 0) return
    from = { at: event.clientX, value: handle.value }
    el.setPointerCapture(event.pointerId)
    hold(true)
    handle.begin()
  })
  el.addEventListener('pointermove', (event) => {
    if (!from) return
    const span = el.getBoundingClientRect().width
    const by = event.clientX - from.at
    handle.move(dragged(handle.row, from.value, by, event.shiftKey, span))
  })
  const stop = () => {
    if (!from) return
    from = null
    hold(false)
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

// The house control: the number is the control. Its name above, the
// value large, and under it a line that shows where the value sits in
// its range. Drag the number or the line. A grip shows on the line
// under the pointer; the ends of the range and a mark at the default
// show only while it is held, when all of it turns purple. A setting
// that rests somewhere inside its range (`opts.bipolar`), like a trim
// or a pan, fills from its default, and keeps the mark there in view.
export const value = (handle, opts = {}) => {
  const rest = opts.bipolar ? handle.row.default : 0
  const out = valueText(handle)
  const line = h('div.kit-line', {},
    h('i.kit-line-fill'),
    h('i.kit-line-default', { style: `--default: ${handle.row.default}` }),
    h('i.kit-line-grip'))
  const scrub = h('div.kit-scrub', {
    tabindex: 0, role: 'slider', 'aria-label': handle.row.title
  }, out, line)
  const range = h('div.kit-range', { 'aria-hidden': 'true' },
    h('span', {}, describe(handle.row, 0)),
    h('span', {}, describe(handle.row, 1)))
  const kind = opts.bipolar ? 'setting.kit-bipolar' : 'setting'
  const el = shell(handle, kind, opts,
    h('span.kit-title', {}, opts.title ?? handle.row.short), scrub, range)
  const show = () => {
    line.style.setProperty('--from', Math.min(rest, handle.value))
    line.style.setProperty('--to', Math.max(rest, handle.value))
    line.style.setProperty('--at', handle.value)
    scrub.setAttribute('aria-valuetext', handle.text)
  }
  scrub.addEventListener('keydown', (event) => {
    if (event.key === 'Enter') out.edit()
  })
  handle.on(show)
  show()
  adjustable(scrub, handle, el, opts.ui)
  return el
}

// A list as pills side by side: the chosen one is filled in.
export const choice = (handle, opts = {}) => {
  const { choices } = handle.row
  const last = Math.max(1, choices.length - 1)
  const buttons = choices.map((name, i) => h('button.kit-option', {
    type: 'button',
    onclick: () => handle.set(i / last)
  }, name))
  const show = () => {
    const at = Math.round(handle.value * last)
    buttons.forEach((b, i) => b.setAttribute('aria-pressed', i === at))
  }
  handle.on(show)
  show()
  return shell(handle, 'choice', opts, h('div.kit-options', {
    role: 'group', 'aria-label': handle.row.title
  }, buttons))
}

// A list as a menu, its choice written as large as a number: for more
// choices than fit side by side.
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
  return shell(handle, 'menu', opts,
    h('span.kit-title', {}, opts.title ?? handle.row.short), select)
}

// On or off: a switch, and beside it what it is and which it is.
export const toggle = (handle, opts = {}) => {
  const said = h('span.kit-switch-said')
  const button = h('button.kit-switch', {
    type: 'button',
    role: 'switch',
    'aria-label': handle.row.title,
    onclick: () => handle.set(handle.value > 0.5 ? 0 : 1)
  }, h('i.kit-switch-thumb'))
  const show = () => {
    const on = handle.value > 0.5
    button.setAttribute('aria-checked', on)
    said.textContent = `${opts.title ?? handle.row.short} ${on ? 'on' : 'off'}`
  }
  handle.on(show)
  show()
  return shell(handle, 'toggle', opts, button, said)
}

// A level: the number is the meter, and under it six segments with
// space between, breaking at -24, -12, -6, -3 and 0 dB. Ink, green,
// yellow, orange, red, and a last one that is the clip lamp: it stays
// lit until clicked. The number takes its segment's colour from yellow
// up, and keeps it for as long as a peak is held.
export const meter = (handle, opts = {}) => {
  const out = h('output.kit-value')
  const segments = ZONES.map(({ name }) =>
    h(`i.kit-segment.kit-zone-${name}`))
  const el = h('div.kit-control.kit-meter', {
    'data-key': handle.row.key,
    'data-meter': handle.row.meter,
    title: 'Click to clear the clip lamp',
    onclick: () => delete el.dataset.clipped
  },
  h('span.kit-title', {}, opts.title ?? handle.row.short),
  out,
  h('div.kit-segments', { role: 'meter', 'aria-label': handle.row.title },
    segments))
  let hold = { level: 0, at: 0 }
  let release = null
  const show = (wait = true) => {
    hold = held(hold, handle.value, performance.now())
    zoneFills(handle.value).forEach((fill, i) => {
      segments[i].style.setProperty('--fill', String(Number(fill.toFixed(3))))
    })
    const [number, unit] = parts(handle.text)
    out.replaceChildren(
      h('span.kit-number', {}, number),
      unit ? h('span.kit-unit', {}, ` ${unit}`) : ''
    )
    el.dataset.zone = zoneOf(hold.level)
    if (handle.value >= 0.999) el.dataset.clipped = 'true'
    clearTimeout(release)
    // Let go of the held peak even if no new reading comes.
    if (wait) release = setTimeout(() => show(false), HOLD_MS + 20)
  }
  handle.on(() => show())
  show(false)
  return el
}

// The control a setting gets when the page does not say: a switch for
// Off and On, pills for a few choices, a menu for many, else a value.
export const control = (handle, opts = {}) => {
  const row = handle.row
  if (row.kind !== 'list') return value(handle, opts)
  const off = row.choices.length === 2 && row.choices[0] === 'Off'
  if (off) return toggle(handle, opts)
  return row.choices.length <= 4 ? choice(handle, opts) : menu(handle, opts)
}

// ---- the page's furniture: a dark glass that holds the picture, and
// a sheet of settings that lies over its lower edge

export const glass = (...children) => h('section.kit-glass', {}, children)

// What stands on the glass under the header: readouts and the picture.
export const stage = (...children) => h('div.kit-stage', {}, children)

// Levels side by side, at the foot of the glass.
export const readouts = (...children) => h('div.kit-readouts', {}, children)

export const sheet = (...children) => h('section.kit-sheet', {}, children)

// The settings on a sheet, four across.
export const values = (...children) => h('div.kit-values', {}, children)

// The colours of the glass, read from the page so a drawing follows
// the tokens.
const inkOf = (el) => {
  const style = getComputedStyle(el)
  const token = (name) => style.getPropertyValue(name).trim()
  return {
    ink: token('--g-ink'),
    soft: token('--g-soft'),
    line: token('--g-line'),
    accent: token('--g-touch'),
    hot: token('--g-red')
  }
}

// The picture: where a plugin draws what it is doing to the sound.
// `draw(pen)` is called whenever a setting or a meter moves or is
// taken hold of; `pen` has the canvas's 2D context as `ctx`, its size
// in CSS pixels as `width` and `height`, the glass's colours as `ink`,
// `soft`, `line`, `accent` and `hot`, and `touched`, the key of the
// setting being dragged now or null: purple is for that alone.
export const picture = (ui, draw, opts = {}) => {
  const canvas = h('canvas')
  const el = h('div.kit-picture', {
    role: 'img', 'aria-label': opts.label ?? `What ${ui.name} is doing`
  }, canvas)
  let queued = false
  const paint = () => {
    queued = false
    const width = canvas.clientWidth
    const height = canvas.clientHeight
    if (!width || !height) return
    const app = document.getElementById('app')
    const zoom = Number(app?.style.zoom) || 1
    const density = (window.devicePixelRatio || 1) * zoom
    canvas.width = Math.round(width * density)
    canvas.height = Math.round(height * density)
    const ctx = canvas.getContext('2d')
    ctx.setTransform(density, 0, 0, density, 0, 0)
    ctx.clearRect(0, 0, width, height)
    draw({ ctx, width, height, touched: ui.touched(), ...inkOf(el) })
  }
  const ask = () => {
    if (queued) return
    queued = true
    requestAnimationFrame(paint)
  }
  ui.store.on(ask)
  ;['resize', 'kit-theme', 'kit-touch', 'kit-density'].forEach((name) =>
    window.addEventListener(name, ask))
  requestAnimationFrame(paint)
  return el
}

// A tool in the header: an icon in a round target, named for a reader.
const tool = (kind, name, label, onclick) => {
  const button = h(`button.kit-tool.kit-${kind}`, { type: 'button', onclick },
    icon(name))
  const say = (text) => {
    button.setAttribute('aria-label', text)
    button.title = text
  }
  say(label)
  return Object.assign(button, { say })
}

// Tools that belong together share a capsule.
const capsule = (...children) => h('div.kit-capsule', {}, children)

// The switch between the light and the dark theme.
export const themeSwitch = () => {
  const label = () => `Switch to the ${otherTheme(currentTheme())} theme`
  const button = tool('theme', 'theme', label(), () => {
    switchTheme()
    button.say(label())
  })
  return button
}

// Roomy, Regular, Dense: each press steps to the next, and the icon
// shows which it is.
export const densitySwitch = () => {
  const label = () => {
    const now = currentDensity()
    return `Density: ${now}. Switch to ${nextDensity(now)}`
  }
  const button = tool('density', currentDensity(), label(), () => {
    const next = switchDensity()
    button.querySelector('path').setAttribute('d', ICONS[next])
    button.say(label())
  })
  return button
}

// Step back and forward through the page's own changes, and flip
// between two sets of settings to hear which is better.
export const tools = (ui) => {
  const { session } = ui
  const undo = tool('undo', 'undo', 'Undo (Ctrl+Z)', session.undo)
  const redo = tool('redo', 'redo', 'Redo (Ctrl+Shift+Z)', session.redo)
  const letters = ['A', 'B'].map((name) => h('span.kit-compare-slot', {}, name))
  const compare = h('button.kit-compare', {
    type: 'button',
    title: 'Compare: keep these settings and flip to the other set',
    onclick: session.flip
  }, letters)
  const show = () => {
    undo.disabled = !session.canUndo
    redo.disabled = !session.canRedo
    letters.forEach((el) => {
      el.dataset.on = String(el.textContent === session.slot)
    })
    const other = session.slot === 'A' ? 'B' : 'A'
    compare.setAttribute('aria-label',
      `Compare: on ${session.slot}, flip to ${other}`)
  }
  session.on(show)
  show()
  return [
    capsule(undo, redo),
    capsule(compare),
    capsule(densitySwitch(), themeSwitch())
  ]
}

// The head of the glass: the plugin's name, a note under it if the
// page has one to make (`opts.note`), and the tools, always in the
// same order so the hand learns them once.
export const header = (ui, opts = {}) => h('header.kit-header', {},
  h('div.kit-heading', {},
    h('h1.kit-name', {}, ui.name),
    opts.note ? h('p.kit-note', {}, opts.note) : null),
  h('div.kit-tools', {}, tools(ui)))

// The foot of the sheet: whatever the page puts there, then what the
// control under the pointer is and does, or how the controls work
// when it is over none.
const HOW = 'Drag a number to change it. Shift for fine. Double-click it ' +
  'to type one, or its line for the default.'

export const foot = (ui, ...children) => {
  const what = h('b.kit-hint-what')
  const about = h('span.kit-hint-about', {}, HOW)
  ui.hint = (title, text) => {
    what.textContent = title ?? ''
    about.textContent = title ? (text || '') : HOW
  }
  return h('footer.kit-foot', {},
    children,
    h('p.kit-hint', { 'aria-live': 'polite', title: ui.build }, what, about))
}

// Every level on the glass, every number on the sheet, every list in
// its foot: a whole page from the spec alone. A start, never the end:
// a plugin's page should show what the plugin does.
export const auto = (ui) => {
  const handles = Object.values(ui.p)
  const lists = handles.filter((handle) => handle.row.kind === 'list')
  const ranges = handles.filter((handle) => handle.row.kind !== 'list')
  const meters = Object.values(ui.m).map((handle) => meter(handle))
  return [
    glass(header(ui), stage(readouts(meters))),
    sheet(
      values(ranges.map((handle) => value(handle, { ui }))),
      foot(ui, lists.map((handle) => control(handle, { ui }))))
  ]
}
