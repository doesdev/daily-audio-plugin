// The editor's side of every plugin: the bridge to the host, a store of
// what the host last said, and the controls a page is composed from.
// Plain ES modules, no framework and no build step. docs/design.md says
// why each thing looks and behaves as it does.
//
// A plugin's main.js composes its page:
//
//   import { mount, header, screen, deck, rule, footer } from './kit/kit.js'
//   mount((ui) => [
//     header(ui),
//     screen(ui, (pen) => { /* draw what the plugin is doing */ }),
//     deck(rule(ui.p.drive), rule(ui.p.tone)),
//     footer(ui)
//   ])
//
// `ui.p` holds a handle for each setting by its key and `ui.m` one for
// each meter. `mount(auto)` lays every setting out with no more said.

export const VERSION = 1

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

// The sweep of a knob for a value, in degrees from straight up.
export const SWEEP = 270
export const angle = (norm) => (clamp(norm) - 0.5) * SWEEP

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

// ---- meters: a peak that holds for a moment, then lets go

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
// and returns a node or a list of them.
export const mount = (build, root = document.getElementById('app')) => {
  startTheme()
  const store = makeStore()
  const session = makeSession(store)
  window.kit = { receive: store.receive, store, session }
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
      // What the foot of the page says about the control under the
      // pointer; `footer` sets it up.
      hint: () => {}
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

// A value as text that can be typed over: double-click it, or press
// Enter on its control. Enter keeps what was typed if the setting can
// hold it; Escape, or anything it cannot hold, leaves it as it was.
const valueText = (handle) => {
  const out = h('output.kit-value', { title: 'Double-click to type a value' })
  const show = () => { if (!out.dataset.editing) out.textContent = handle.text }
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

// What every control shares: its title, its body and, unless the
// control shows its own value (`opts.value` false), that value as text.
const labelled = (handle, kind, body, opts = {}) => {
  const out = opts.value === false ? null : valueText(handle)
  const tone = opts.tone ? `.kit-tone-${opts.tone}` : ''
  const el = h(`div.kit-control.kit-${kind}${tone}`, {
    'data-key': handle.row.key,
    'data-id': handle.row.id
  }, h('span.kit-title', {}, opts.title ?? handle.row.short), body, out)
  if (out) {
    body.addEventListener('keydown', (event) => {
      if (event.key === 'Enter') out.edit()
    })
  }
  return hinted(el, opts.ui, handle)
}

// Dragging, the wheel, the arrow keys and double-click to reset, on
// any element that stands for a setting's value. A drag is up and down
// unless `across`, when it is along the element's own length.
const adjustable = (el, handle, across = false) => {
  const along = (event) => (across ? event.clientX : -event.clientY)
  let from = null
  el.addEventListener('pointerdown', (event) => {
    if (event.button !== 0) return
    from = { at: along(event), value: handle.value }
    el.setPointerCapture(event.pointerId)
    el.dataset.held = 'true'
    handle.begin()
  })
  el.addEventListener('pointermove', (event) => {
    if (!from) return
    const span = across ? el.getBoundingClientRect().width : 160
    const by = along(event) - from.at
    handle.move(dragged(handle.row, from.value, by, event.shiftKey, span))
  })
  const stop = () => {
    if (!from) return
    from = null
    delete el.dataset.held
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

// The house control: a ruled slot cut into the panel, a raised grip
// that rides in it, the setting's title before and its value after.
// The rule's marks are the setting's own: a mark at its default, and
// the travel from where it rests to where it is, filled.
export const rule = (handle, opts = {}) => {
  const rest = opts.bipolar ? 0.5 : 0
  const slot = h('div.kit-rule-slot', {
    tabindex: 0,
    role: 'slider',
    'aria-label': handle.row.title
  },
  h('div.kit-rule-marks'),
  h('div.kit-rule-default', { style: `--default: ${handle.row.default}` }),
  h('div.kit-rule-fill'),
  h('div.kit-rule-grip'))
  const show = () => {
    slot.style.setProperty('--from', Math.min(rest, handle.value))
    slot.style.setProperty('--to', Math.max(rest, handle.value))
    slot.style.setProperty('--at', handle.value)
    slot.setAttribute('aria-valuetext', handle.text)
  }
  handle.on(show)
  show()
  return labelled(handle, 'rule', adjustable(slot, handle, true), opts)
}

// A rotary, for the one or two settings that deserve the room: a
// turned cap standing off the panel, lit from above, its pointer the
// only mark of colour. The scale has three marks: least, default, most.
export const knob = (handle, opts = {}) => {
  const mark = (norm, name) => h(`line.kit-knob-mark${name}`, {
    x1: 24, y1: 2.5, x2: 24, y2: 5.5, transform: `rotate(${angle(norm)} 24 24)`
  })
  const pointer = h('line.kit-knob-pointer', { x1: 24, y1: 11, x2: 24, y2: 18 })
  const dial = h('svg.kit-knob-dial', {
    viewBox: '0 0 48 48',
    tabindex: 0,
    role: 'slider',
    'aria-label': handle.row.title
  },
  mark(0, ''), mark(1, ''), mark(handle.row.default, '.kit-knob-default'),
  h('circle.kit-knob-skirt', { cx: 24, cy: 24, r: 17 }),
  h('circle.kit-knob-cap', { cx: 24, cy: 24, r: 14.5 }),
  pointer)
  const show = () => {
    pointer.setAttribute('transform', `rotate(${angle(handle.value)} 24 24)`)
    dial.setAttribute('aria-valuetext', handle.text)
  }
  handle.on(show)
  show()
  return labelled(handle, 'knob', adjustable(dial, handle), opts)
}

// A list as keys side by side in one slot: the chosen one stands up.
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
  const button = h('button.kit-switch', {
    type: 'button',
    role: 'switch',
    'aria-label': handle.row.title,
    onclick: () => handle.set(handle.value > 0.5 ? 0 : 1)
  }, h('span.kit-switch-thumb'))
  const show = () => button.setAttribute('aria-checked', handle.value > 0.5)
  handle.on(show)
  show()
  return labelled(handle, 'toggle', button, { ...opts, value: false })
}

// A level, read the way an engineer reads one: a column of ink in a
// slot against a scale, the peak held as a line for a moment after it
// passes, and the level as a number beneath. It is ink, not colour,
// until the level is one to act on: the top 6 dB warm, and a clip
// lights a lamp that stays lit until it is clicked.
const SCALE_DB = [0, -6, -12, -24, -36, -48]

export const meter = (handle, opts = {}) => {
  const level = handle.row.meter === 'level'
  const out = h('output.kit-value')
  const marks = level
    ? SCALE_DB.map((db) => h('div.kit-meter-mark', {
      style: `--at: ${1 + db / 60}`
    }))
    : []
  const slot = h('div.kit-meter-slot', {
    role: 'meter',
    'aria-label': handle.row.title
  }, marks, h('div.kit-meter-fill'), h('div.kit-meter-peak'))
  const tone = opts.tone ? `.kit-tone-${opts.tone}` : ''
  const el = h(`div.kit-control.kit-meter${tone}`, {
    'data-key': handle.row.key,
    'data-meter': handle.row.meter,
    title: level ? 'Click to clear the clip lamp' : null,
    onclick: () => delete el.dataset.clipped
  },
  h('span.kit-title', {}, opts.title ?? handle.row.short),
  h('div.kit-meter-lamp'),
  slot,
  out)
  let hold = { level: 0, at: 0 }
  let release = null
  const show = (wait = true) => {
    hold = held(hold, handle.value, performance.now())
    slot.style.setProperty('--level', handle.value)
    slot.style.setProperty('--peak', hold.level)
    out.textContent = handle.text
    if (level && handle.value >= 0.999) el.dataset.clipped = 'true'
    clearTimeout(release)
    // Let go of the held peak even if no new reading comes.
    if (wait) release = setTimeout(() => show(false), HOLD_MS + 20)
  }
  handle.on(() => show())
  show(false)
  return el
}

// The control a setting gets when the page does not say: a switch for
// Off and On, keys for a few choices, a menu for many, else a rule.
export const control = (handle, opts = {}) => {
  const row = handle.row
  if (row.kind !== 'list') return rule(handle, opts)
  const off = row.choices.length === 2 && row.choices[0] === 'Off'
  if (off) return toggle(handle, opts)
  return row.choices.length <= 4 ? choice(handle, opts) : menu(handle, opts)
}

// ---- the page's furniture

export const row = (...children) => h('div.kit-row', {}, children)

export const column = (...children) => h('div.kit-column', {}, children)

// A raised panel that controls sit on.
export const deck = (...children) => h('section.kit-deck', {}, children)

// Controls that belong together, under a name.
export const group = (title, ...children) => h('div.kit-group', {},
  title ? h('h2.kit-group-title', {}, title) : null,
  children)

// The side of the page where the signal leaves: its meters, and the
// setting that trims it.
export const rail = (...children) => h('aside.kit-rail', {}, children)

// The stage: the screen and what stands beside it.
export const stage = (...children) => h('div.kit-stage', {}, children)

// The colours of the screen, read from the page so a drawing follows
// the tokens.
const inkOf = (el) => {
  const style = getComputedStyle(el)
  const token = (name) => style.getPropertyValue(name).trim()
  return {
    ink: token('--screen-ink'),
    soft: token('--screen-soft'),
    line: token('--screen-line'),
    accent: token('--screen-accent'),
    hot: token('--hot')
  }
}

// The screen: a dark glass set into the body, in either theme, where
// a plugin draws what it is doing to the sound. `draw(pen)` is called
// whenever a setting or a meter moves; `pen` has the canvas's 2D
// context as `ctx`, its size in CSS pixels as `width` and `height`,
// and the screen's colours as `ink`, `soft`, `line`, `accent`, `hot`.
export const screen = (ui, draw, opts = {}) => {
  const canvas = h('canvas.kit-screen-canvas')
  const el = h('div.kit-screen', {
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
    draw({ ctx, width, height, ...inkOf(el) })
  }
  const ask = () => {
    if (queued) return
    queued = true
    requestAnimationFrame(paint)
  }
  ui.store.on(ask)
  window.addEventListener('resize', ask)
  window.addEventListener('kit-theme', ask)
  requestAnimationFrame(paint)
  return el
}

// The switch between the light and the dark theme: a disc, half full.
export const themeSwitch = () => {
  const label = () => `Switch to the ${otherTheme(currentTheme())} theme`
  const button = h('button.kit-tool.kit-theme', {
    type: 'button',
    onclick: () => {
      switchTheme()
      button.setAttribute('aria-label', label())
      button.title = label()
    }
  }, h('svg', { viewBox: '0 0 16 16', 'aria-hidden': 'true' },
    h('circle', { cx: 8, cy: 8, r: 5.6 }),
    h('path.kit-fill', { d: 'M 8 2.4 A 5.6 5.6 0 0 1 8 13.6 Z' })))
  button.setAttribute('aria-label', label())
  button.title = label()
  return button
}

// Step back and forward through the page's own changes, and flip
// between two sets of settings to hear which is better.
export const tools = (ui) => {
  const { session } = ui
  const icon = (d) => h('svg', { viewBox: '0 0 16 16', 'aria-hidden': 'true' },
    h('path', { d }))
  const undo = h('button.kit-tool.kit-undo', {
    type: 'button',
    title: 'Undo (Ctrl+Z)',
    'aria-label': 'Undo',
    onclick: session.undo
  }, icon('M 6.5 4 L 3 7.5 L 6.5 11 M 3 7.5 H 10 A 3 3 0 0 1 10 13.5 H 8'))
  const redo = h('button.kit-tool.kit-redo', {
    type: 'button',
    title: 'Redo (Ctrl+Shift+Z)',
    'aria-label': 'Redo',
    onclick: session.redo
  }, icon('M 9.5 4 L 13 7.5 L 9.5 11 M 13 7.5 H 6 A 3 3 0 0 0 6 13.5 H 8'))
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
  return h('div.kit-tools', {}, undo, redo, compare)
}

export const header = (ui, ...children) => h('header.kit-header', {},
  h('h1.kit-name', {}, ui.name),
  children,
  h('div.kit-header-end', {}, tools(ui), themeSwitch()))

// The foot of the page: what the control under the pointer is and
// does, or how the controls work when it is over none.
const HOW = 'Drag to set. Shift for fine. Double-click a control for ' +
  'its default, or its value to type one.'

export const footer = (ui) => {
  const what = h('span.kit-hint-what')
  const about = h('span.kit-hint-about', {}, HOW)
  ui.hint = (title, text) => {
    what.textContent = title ?? ''
    about.textContent = title ? (text || '') : HOW
  }
  return h('footer.kit-footer', {},
    h('div.kit-hint', { 'aria-live': 'polite' }, what, about),
    h('span.kit-build', {}, ui.build))
}

// Every setting on a deck under its group's name, the meters on a
// rail: a whole page from the spec alone. A start, never the end: a
// plugin's page should show what the plugin does.
export const auto = (ui) => {
  const handles = Object.values(ui.p)
  const groups = [...new Set(handles.map((handle) => handle.row.group))]
  const meters = Object.values(ui.m).map((handle) => meter(handle))
  return [
    header(ui),
    stage(
      deck(groups.map((name) => group(name, handles
        .filter((handle) => handle.row.group === name)
        .map((handle) => control(handle, { ui }))))),
      meters.length ? rail(row(meters)) : null),
    footer(ui)
  ]
}
