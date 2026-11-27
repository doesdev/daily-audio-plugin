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

// A setting with a few steps, each worth feeling: a choice, a count.
// One with hundreds (a frequency that lands on whole hertz) is a range
// that happens to round, and moves like any other range.
const NOTCHED = 24
const notched = (row) => row.steps > 0 && row.steps <= NOTCHED

// Where a drag of `by` pixels from `from` lands on a control `span`
// pixels long: the whole range in one span, or 24 pixels a step where
// the steps are few. A fine drag (shift) moves a tenth as far.
export const dragged = (row, from, by, fine = false, span = 160) => {
  const length = notched(row) ? Math.max(span, row.steps * 24) : span
  return clamp(from + by / (fine ? length * 10 : length))
}

// One notch of the wheel or an arrow key: a step where the steps are
// few, else a fiftieth of the range (a five-hundredth when fine), and
// never less than one step.
export const nudged = (row, from, by, fine = false) => {
  const least = row.steps > 0 ? 1 / row.steps : 0
  const unit = notched(row) ? least : Math.max(least, fine ? 0.002 : 0.02)
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

// A finished gesture: `index` went from `before` to `after`. Several
// settings moved as one (a preset loaded) are a `batch` of those, and
// step back and forward as one.
export const recorded = (history, change) => {
  const nothing = change.batch
    ? !change.batch.length
    : change.before === change.after
  return nothing
    ? history
    : { past: [...history.past, change].slice(-100), future: [] }
}

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

export const HOLD_MS = 2000

// How many readings of the meters are kept: about four seconds' worth
// at the 30 a second the host sends.
export const TRAIL = 120

// The held peak after a reading of `level` at time `now`.
export const held = (hold, level, now, ms = HOLD_MS) =>
  level >= hold.level || now - hold.at > ms ? { level, at: now } : hold

// ---- two settings that move as one

// Where the other of a pair goes when its lead has moved from where
// the two were taken hold of: alongside it, against it, or to
// whatever the lead leaves of the whole.
export const TIES = {
  linked: (start, lead) => clamp(start.other + lead - start.lead),
  mirrored: (start, lead) => clamp(start.other - (lead - start.lead)),
  opposed: (start, lead) => clamp(1 - lead)
}

// ---- presets: named sets of settings

// A preset's settings, given by key in their own terms (a number, a
// choice's name), as a value for every setting: one it does not name
// stays at its default.
export const presetValues = (params, values = {}) => params.map((row) => {
  const given = values[row.key]
  if (given == null) return row.default
  if (row.kind !== 'list') return snap(row, normOf(row, given))
  const at = typeof given === 'number' ? given : row.choices.indexOf(given)
  return at < 0 ? row.default : normOf(row, at)
})

// What a preset looks like at a glance: its first four numbers.
export const fingerprint = (params, values) => params
  .map((row, i) => [row, values[i]])
  .filter(([row]) => row.kind !== 'list')
  .slice(0, 4)
  .map(([, value]) => value)

// The settings a preset would move from where they are now, leaving
// the locked ones alone: [index, before, after].
export const presetMoves = (now, to, locked = []) => to
  .map((after, index) => [index, now[index], after])
  .filter(([index, before, after]) =>
    before !== after && !locked.includes(index))

// ---- what the person has chosen for every editor of ours

export const PREFS = {
  hold: ['Off', '2 s', 'Until pressed'],
  motion: ['Full', 'Still'],
  fine: ['With Shift', 'Always'],
  fall: ['Fast', 'Medium', 'Slow']
}

export const PREF_DEFAULTS = {
  hold: '2 s', motion: 'Full', fine: 'With Shift', fall: 'Medium'
}

// Saved choices over the defaults, dropping any no longer offered.
export const prefsFor = (saved) => Object.fromEntries(
  Object.entries(PREF_DEFAULTS).map(([key, standard]) =>
    [key, PREFS[key].includes(saved?.[key]) ? saved[key] : standard])
)

// How long a meter holds its peak, in milliseconds.
export const holdFor = (choice) =>
  ({ Off: 0, 'Until pressed': Infinity })[choice] ?? HOLD_MS

// ---- the sound itself, for a picture to draw

// Where to start drawing `samples` so a steady wave stands still: the
// first place it climbs through zero that leaves `span` samples after
// it, or the start if it never does.
export const steady = (samples, span) => {
  const last = samples.length - span
  const at = samples.findIndex((v, i) =>
    i > 0 && i <= last && samples[i - 1] <= 0 && v > 0)
  return at < 0 ? 0 : at
}

// The spectrum of `samples`: how loud each band is, in dB against a
// full-scale sine, for the first half of as many bands as the largest
// power of two of samples. Band `k` is at `k * rate / size` Hz.
export const spectrum = (samples) => {
  const size = 2 ** Math.floor(Math.log2(Math.max(2, samples.length)))
  const from = samples.length - size
  // A Hann window, so a tone between two bands does not smear.
  const re = Float64Array.from({ length: size }, (_, i) =>
    (samples[from + i] ?? 0) * (0.5 - 0.5 * Math.cos(2 * Math.PI * i / size)))
  const im = new Float64Array(size)
  // In place, as the transform is always written: bit-reversed order
  // first, then butterflies of doubling width.
  for (let i = 1, j = 0; i < size; i++) {
    let bit = size >> 1
    for (; j & bit; bit >>= 1) j ^= bit
    j ^= bit
    if (i < j) [re[i], re[j]] = [re[j], re[i]]
  }
  for (let width = 2; width <= size; width *= 2) {
    const turn = -2 * Math.PI / width
    for (let start = 0; start < size; start += width) {
      for (let k = 0; k < width / 2; k++) {
        const [cos, sin] = [Math.cos(turn * k), Math.sin(turn * k)]
        const [a, b] = [start + k, start + k + width / 2]
        const [xr, xi] = [re[b] * cos - im[b] * sin, re[b] * sin + im[b] * cos]
        ;[re[b], im[b]] = [re[a] - xr, im[a] - xi]
        ;[re[a], im[a]] = [re[a] + xr, im[a] + xi]
      }
    }
  }
  // A full-scale sine through a Hann window peaks at a quarter of the
  // size; that is 0 dB.
  return Array.from({ length: size / 2 }, (_, k) =>
    20 * Math.log10(Math.max(1e-9, Math.hypot(re[k], im[k]) / (size / 4))))
}

// What a setting says of itself beside its name: the controller it has
// learned, that it is waiting for one, that presets leave it alone.
export const tagFor = (id, learn, locked) => {
  const cc = learn?.map[id]
  const said = [
    learn?.waiting === id ? 'move a controller' : null,
    cc == null ? null : `CC ${cc}`,
    locked ? 'locked' : null
  ].filter(Boolean)
  return said.length ? ` (${said.join(', ')})` : ''
}

// How fast a meter's bar falls, as a share of its 60 dB a second: it
// rises at once and comes down at this pace, so it can be read.
export const fallFor = (choice) =>
  ({ Fast: 40, Slow: 10 }[choice] ?? 20) / 60

// Where a falling bar is `seconds` later: never under the level now.
export const fallen = (shown, level, seconds, rate) =>
  Math.max(level, shown - rate * Math.max(0, seconds))

// The size, as a share of its full size, at which text `wide` across
// fits in `room`: never under `least`, where it is cut short instead.
export const textFit = (wide, room, least = 0.6) =>
  wide <= room || wide <= 0 ? 1 : Math.max(least, room / wide)

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
  levelTexts: [],
  // The person's own presets as the host keeps them, in files; null
  // until it says, and for a host that keeps none.
  files: null,
  // The sound itself, while audio runs: { rate, input, output }, the
  // latest samples each way, oldest first. Null until the host sends.
  signal: null,
  // MIDI learn, where the host side has it: { waiting, map }, the id
  // of the setting waiting for a controller and each setting's
  // controller by its id. Null until the host says.
  learn: null,
  // Linking, where the host side has it: { me, groups, peers }. This
  // instance's number, the group of each linked setting by its id,
  // and every other instance as { uid, name, keys: { key: group } }.
  links: null
}

export const reduce = (state, msg) => {
  if (msg.type === 'hello') {
    const { type, version, ...rest } = msg
    return { ...EMPTY, ...rest, ready: true }
  }
  if (msg.type === 'state') {
    return { ...state, values: msg.values, texts: msg.texts }
  }
  if (msg.type === 'signal') {
    const real = (samples) => samples.map((v) => v / msg.scale)
    return {
      ...state,
      signal: {
        rate: msg.rate, input: real(msg.input), output: real(msg.output)
      }
    }
  }
  if (msg.type === 'learn') {
    return { ...state, learn: { waiting: msg.waiting, map: msg.map ?? {} } }
  }
  if (msg.type === 'links') {
    const { me, groups = {}, peers = [] } = msg
    return { ...state, links: { me, groups, peers } }
  }
  if (msg.type === 'presets') {
    return { ...state, files: msg.list, filesDir: msg.dir }
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
    const moves = change.batch ?? [change]
    moves.forEach((move) => put(move.index, pick(move)))
    tell()
  }
  const locked = new Set()
  return {
    on: (fn) => { listeners.add(fn); return () => listeners.delete(fn) },
    record: (change) => { history = recorded(history, change); tell() },
    undo: () => step(undone(history), (change) => change.before),
    redo: () => step(redone(history), (change) => change.after),
    get canUndo () { return history.past.length > 0 },
    get canRedo () { return history.future.length > 0 },
    get slot () { return slot },
    put,
    // Every setting to `to` as one step, but for the locked ones.
    apply: (to) => {
      const moves = presetMoves(store.get().values, to, [...locked])
      moves.forEach(([index, , after]) => put(index, after))
      history = recorded(history, {
        batch: moves.map(([index, before, after]) => ({ index, before, after }))
      })
      tell()
    },
    // A locked setting stays where it is when a preset is loaded.
    lock: (index, on) => {
      if (on) locked.add(index)
      else locked.delete(index)
      tell()
    },
    isLocked: (index) => locked.has(index),
    get locked () { return [...locked] },
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

const PREFS_KEY = 'kit-prefs'

const readPrefs = () => {
  try {
    return prefsFor(JSON.parse(window.localStorage.getItem(PREFS_KEY)))
  } catch { return prefsFor(null) }
}

let prefs = prefsFor(null)

const applyPrefs = () => {
  document.documentElement.dataset.motion = prefs.motion.toLowerCase()
}

const setPref = (key, choice) => {
  prefs = prefsFor({ ...prefs, [key]: choice })
  try { window.localStorage.setItem(PREFS_KEY, JSON.stringify(prefs)) } catch {}
  applyPrefs()
}

// Back to whatever the system is set to, forgetting the choice.
const matchTheme = () => {
  try { window.localStorage.removeItem(THEME_KEY) } catch {}
  startTheme()
  window.dispatchEvent(new Event('kit-theme'))
}

const setDensity = (density) => {
  document.documentElement.dataset.density = density
  try { window.localStorage.setItem(DENSITY_KEY, density) } catch {}
  window.dispatchEvent(new Event('kit-density'))
}

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

const setTheme = (theme) => {
  applyTheme(theme)
  try { window.localStorage.setItem(THEME_KEY, theme) } catch {}
  window.dispatchEvent(new Event('kit-theme'))
  return theme
}

const switchTheme = () => setTheme(otherTheme(currentTheme()))

// How the page fills a window of any shape. It is scaled, whole, by
// as much as the tighter of the two directions allows, so nothing gets
// small; then it is laid out as wide and as tall as the window is at
// that scale, so a window that is not the design's shape is filled:
// the glass and its picture take whatever is left over, and no band of
// empty window is left beside or below.
export const fitted = (size, base) => {
  const zoom = fitScale(size, base)
  return { zoom, width: size[0] / zoom, height: size[1] / zoom }
}

const fitTo = (root, base) => {
  const fit = () => {
    const to = fitted([window.innerWidth, window.innerHeight], base)
    root.style.width = `${to.width}px`
    root.style.height = `${to.height}px`
    root.style.zoom = to.zoom
  }
  window.addEventListener('resize', fit)
  fit()
}

// Builds the page once the host says hello. `build` gets the handles
// and returns a node or a list of them. `opts.density` is the density
// the plugin starts in until the person picks one, and `opts.presets`
// the presets it ships: [{ name, values: { key: value in its own
// terms } }].
export const mount = (build, opts = {}) => {
  const root = opts.root ?? document.getElementById('app')
  startTheme()
  startDensity(opts.density)
  prefs = readPrefs()
  applyPrefs()
  let touched = null
  // The last few seconds of every meter, a reading a tick.
  const trail = []
  const store = makeStore()
  const session = makeSession(store)
  window.kit = {
    receive: store.receive, store, session, touched: () => touched, trail
  }
  store.on((state, prev) => {
    if (state.levels === prev.levels || !prev.ready) return
    trail.push(state.levels)
    if (trail.length > TRAIL) trail.shift()
  })
  // Escape puts an open panel away, and only that: the keyboard goes
  // back to the host on the next one.
  window.addEventListener('keydown', (event) => {
    if (event.key !== 'Escape') return
    const open = document.querySelector('.kit-sheet[data-panel]')
    const waiting = store.get().learn?.waiting != null
    if (!open && !waiting) return
    event.stopImmediatePropagation()
    // A setting waiting for a controller stops waiting.
    if (open) closePanel()
    else post({ type: 'learn', id: null })
  })
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
      presets: makePresets(store, session, state.name, opts.presets ?? []),
      // What the foot of the sheet says about the control under the
      // pointer; `foot` sets it up.
      hint: () => {},
      // The key of the setting being dragged now, for the picture to
      // show in purple; a control says when it is taken and let go.
      touched: () => touched,
      // A meter's readings over the last few seconds, oldest first:
      // the one live thing the page hears of the sound itself.
      trail: (handle) => trail.map((levels) => levels[handle.index] ?? 0),
      touch: (key) => {
        touched = key
        window.dispatchEvent(new Event('kit-touch'))
      },
      // Says something on the glass in place of the picture, until it
      // is answered: `notice`'s options, or nothing to take it away.
      notify: (said) => {
        const stageEl = document.querySelector('.kit-stage')
        stageEl?.querySelector(':scope > .kit-notice')?.remove()
        if (stageEl) delete stageEl.dataset.noticed
        if (!said || !stageEl) return
        stageEl.dataset.noticed = 'true'
        stageEl.append(notice({
          ...said,
          actions: (said.actions ?? []).map((action) => ({
            ...action, run: () => { ui.notify(null); action.run?.() }
          }))
        }))
      }
    }
    ui.presets.ask = (preset, go) => {
      closePanel()
      const names = preset.unknown.join(', ')
      const count = preset.unknown.length
      ui.notify({
        tone: 'Nothing has changed yet',
        title: `${preset.name} was made by a newer version`,
        text: `${count} of its settings ${count === 1 ? 'is' : 'are'} ` +
          `not known here: ${names}.`,
        actions: [
          { name: 'Load what fits', main: true, run: go },
          { name: 'Cancel' }
        ]
      })
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
  // The browser's own menu is never ours: where a right-click has a
  // menu it has opened by now, and anywhere else nothing opens. A box
  // being typed in keeps its own, for its copy and paste.
  window.addEventListener('contextmenu', (event) => {
    if (!event.target.closest?.('input, textarea')) event.preventDefault()
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
  theme: 'M10 4a6 6 0 1 0 0 12a6 6 0 1 0 0-12M10 4v12',
  // Two sliders: settings. Not a sun, which would say brightness.
  settings: 'M4 6.5h6M14 6.5h2M12 4.7a1.8 1.8 0 1 0 0 3.6a1.8 1.8 0 1 0 ' +
    '0-3.6M4 13.5h2M10 13.5h6M8 11.7a1.8 1.8 0 1 0 0 3.6a1.8 1.8 0 1 0 0-3.6',
  help: 'M7.4 7.6a2.7 2.7 0 1 1 3.9 2.4c-.8.5-1.3 1-1.3 2M10 14.6v.2',
  power: 'M10 3.5v6M6.2 5.8a5.5 5.5 0 1 0 7.6 0',
  chevron: 'M6 8l4 4 4-4',
  match: 'M5 8h10M5 12h10',
  learn: 'M10 4.5a5.5 5.5 0 1 0 0 11a5.5 5.5 0 1 0 0-11M10 10l2.6-2.6',
  chosen: 'M5 10.5l3.2 3.2L15 7',
  blank: '',
  close: 'M5.5 5.5l9 9M14.5 5.5l-9 9',
  type: 'M4 15.5h12M6 12l1-3.5 6-6 2.5 2.5-6 6z',
  reset: 'M5 10a5 5 0 1 0 1.6-3.7M5 3.5v3h3',
  copy: 'M7.5 7.5h8v8h-8zM4.5 12.5v-8h8',
  paste: 'M6 4.5h8v11H6zM8 4.5v-1.5h4v1.5M8.5 9h3M8.5 12h3',
  lock: 'M5.5 9.5h9v6h-9zM7.5 9.5V7a2.5 2.5 0 0 1 5 0v2.5',
  link: 'M8.5 11.5l3-3M9 6.5l1.2-1.2a3 3 0 0 1 4.3 4.3L13.3 11M11 ' +
    '13.5l-1.2 1.2a3 3 0 0 1-4.3-4.3L6.7 9'
}

export const icon = (name) => h('svg.kit-icon', {
  viewBox: '0 0 20 20', 'aria-hidden': 'true'
}, h('path', { d: ICONS[name] }))

// The page's own hint for a control: what it is and what it does.
// The one tooltip: the house's own, never the browser's. A short name
// after a moment over a tool; with help on, sooner and with more.
let tip = null
let tipTimer = null

const hideTip = () => {
  clearTimeout(tipTimer)
  tip?.remove()
  tip = null
}

const showTip = (anchor, said) => {
  hideTip()
  const app = document.getElementById('app')
  if (!app || !anchor.isConnected) return
  tip = h('div.kit-tip', { role: 'tooltip' },
    said.title ? h('b', {}, said.title) : null,
    said.text ? h('span', {}, said.text) : null)
  app.append(tip)
  const zoom = Number(app.style.zoom) || 1
  const frame = app.getBoundingClientRect()
  const box = anchor.getBoundingClientRect()
  const centre = ((box.left + box.right) / 2 - frame.left) / zoom
  const below = (box.bottom - frame.top) / zoom + 8
  const above = (box.top - frame.top) / zoom - tip.offsetHeight - 8
  const left = clamp(centre - tip.offsetWidth / 2, 8,
    app.clientWidth - tip.offsetWidth - 8)
  const fits = below + tip.offsetHeight < app.clientHeight - 8
  tip.style.left = `${left}px`
  tip.style.top = `${fits ? below : Math.max(8, above)}px`
}

// Gives `el` a tooltip: `say(help)` is `{ title, text }`, or nothing
// for none; `help` is whether help is on. A press puts it away.
const tipped = (el, say) => {
  const open = () => {
    const help = document.documentElement.dataset.help === 'true'
    clearTimeout(tipTimer)
    tipTimer = setTimeout(() => {
      const said = say(help)
      if (said) showTip(el, said)
    }, help ? 120 : 450)
  }
  el.addEventListener('pointerenter', open)
  el.addEventListener('focusin', () => {
    if (el.matches(':focus-visible')) open()
  })
  ;['pointerleave', 'pointerdown', 'focusout'].forEach((name) =>
    el.addEventListener(name, hideTip))
  return el
}

// The foot of the sheet says what a control is while the pointer is
// over it or the keyboard is on it, and stops only when neither is.
// With help on, a card beside the control says the rest.
const hinted = (el, ui, handle, card = true) => {
  if (!ui) return el
  let over = false
  let focused = false
  const tell = () => (over || focused
    ? ui.hint(handle.row.title, handle.row.about, handle.row)
    : ui.hint())
  el.addEventListener('pointerenter', () => { over = true; tell() })
  el.addEventListener('pointerleave', () => { over = false; tell() })
  el.addEventListener('focusin', () => { focused = true; tell() })
  el.addEventListener('focusout', () => { focused = false; tell() })
  // A tool has its own tip already.
  if (!card) return el
  return tipped(el, (help) => (help
    ? {
        title: handle.row.title,
        text: `${stopped(handle.row.about)}${helpFor(handle.row)}`
      }
    : null))
}

// A sentence with its full stop, for when another follows it.
const stopped = (text) =>
  (text && !/[.!?]$/.test(text) ? `${text}.` : text || '')

// Shrinks `el`'s text until it fits its box, down to `least` pixels;
// past that the box cuts it short. The box never grows: nothing else
// on the page moves because of what a line of text says.
const fitText = (el, most = 13, least = 10.5) => {
  let size = most
  el.style.fontSize = `${size}px`
  while (size > least && el.scrollHeight > el.clientHeight + 1) {
    size -= 0.5
    el.style.fontSize = `${size}px`
  }
}

// A value as its number drawn large and its unit small, which can be
// typed over: double-click it, or press Enter on its control. Enter
// keeps what was typed if the setting can hold it; Escape, or anything
// it cannot hold, leaves it as it was.
const valueText = (handle) => {
  const out = h('output.kit-value')
  // A number too wide for its column is drawn smaller, not let run
  // into the next one.
  const fit = () => {
    out.style.setProperty('--fit', 1)
    const number = out.querySelector('.kit-number')
    if (!out.isConnected || !number) return
    // Only the number is scaled: its unit keeps its size and its room.
    const wide = (el) => el?.getBoundingClientRect().width ?? 0
    const room = wide(out) - wide(out.querySelector('.kit-unit')) - 1
    out.style.setProperty('--fit', textFit(wide(number), room).toFixed(3))
  }
  const show = () => {
    if (out.dataset.editing) return
    const [number, unit] = parts(handle.text)
    out.replaceChildren(
      h('span.kit-number', {}, number),
      unit ? h('span.kit-unit', {}, ` ${unit}`) : ''
    )
    fit()
  }
  ;['resize', 'kit-density', 'kit-panel'].forEach((name) =>
    window.addEventListener(name, () => requestAnimationFrame(fit)))
  requestAnimationFrame(fit)
  const edit = (start) => {
    if (out.dataset.editing) return
    out.dataset.editing = 'true'
    const input = h('input.kit-entry', {
      type: 'text', value: start ?? handle.text, 'aria-label': handle.row.title
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
    if (start == null) return input.select()
    // What was typed stays, and what is typed next goes after it.
    input.focus()
    input.setSelectionRange(start.length, start.length)
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
const shell = (handle, kind, opts, ...children) => {
  const el = h(`div.kit-control.kit-${kind}`, {
    'data-key': handle.row.key, 'data-id': handle.row.id
  }, children)
  const { ui } = opts
  if (ui) {
    // A control that has no menu of its own still has this one.
    // A value opens its own, fuller one, and marks the event: this
    // waits a moment to see, since either may hear the click first.
    el.addEventListener('contextmenu', (event) => {
      event.preventDefault()
      setTimeout(() => {
        const item = !event.kitMenu && linkItem(ui, handle.row.title, [handle])
        if (item) popup([item], [event.clientX, event.clientY])
      }, 0)
    })
    // And says so while it is linked to another instance.
    const show = () => {
      const linked = linkedTo(ui.store.get().links, [handle.row]).length > 0
      el.toggleAttribute('data-linked', linked)
    }
    ui.store.on((state, prev) => { if (state.links !== prev.links) show() })
    show()
  }
  return hinted(el, ui, handle)
}

// Dragging along it, the wheel, the arrow keys and double-click for
// the default, on the element that stands for a setting's value.
// `mark` is the control that wears the purple while it is held.
const adjustable = (el, handle, mark, ui, upright = false) => {
  const along = (event) => (upright ? -event.clientY : event.clientX)
  let from = null
  const hold = (on) => {
    if (on) mark.dataset.held = 'true'
    else delete mark.dataset.held
    ui?.touch(on ? handle.row.key : null)
  }
  el.addEventListener('pointerdown', (event) => {
    if (event.button !== 0) return
    from = { at: along(event), value: handle.value }
    el.setPointerCapture(event.pointerId)
    hold(true)
    handle.begin()
  })
  el.addEventListener('pointermove', (event) => {
    if (!from) return
    const box = el.getBoundingClientRect()
    const span = upright ? box.height : box.width
    const fine = event.shiftKey || prefs.fine === 'Always'
    const by = along(event) - from.at
    handle.move(dragged(handle.row, from.value, by, fine, span))
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
    // Delete is the default, as it is in the hosts people know.
    if (event.key === 'Delete' || event.key === 'Backspace') {
      handle.set(handle.row.default)
    }
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
  const tag = h('span.kit-tag')
  const el = shell(handle, kind, opts,
    h('span.kit-title', {}, opts.title ?? handle.row.short, tag), scrub, range)
  const show = () => {
    line.style.setProperty('--from', Math.min(rest, handle.value))
    line.style.setProperty('--to', Math.max(rest, handle.value))
    line.style.setProperty('--at', handle.value)
    scrub.setAttribute('aria-valuetext', handle.text)
  }
  scrub.addEventListener('keydown', (event) => {
    const mod = event.ctrlKey || event.metaKey
    if (event.key === 'Enter') out.edit()
    // A digit, a sign or a point starts a value, with no click first.
    else if (!mod && /^[\d.+-]$/.test(event.key)) {
      event.preventDefault()
      out.edit(event.key)
    } else if (mod && event.key.toLowerCase() === 'c') copyValue(handle)
    else if (mod && event.key.toLowerCase() === 'v') pasteValue(handle)
  })
  el.addEventListener('contextmenu', (event) => {
    event.preventDefault()
    event.kitMenu = true
    valueMenu(el, handle, out, opts.ui, [event.clientX, event.clientY])
  })
  if (opts.ui) {
    const { session, store } = opts.ui
    const mark = () => {
      const locked = session.isLocked(handle.index)
      const { learn } = store.get()
      const flag = (name, on) => {
        if (on) el.dataset[name] = 'true'
        else delete el.dataset[name]
      }
      flag('locked', locked)
      flag('learning', learn?.waiting === handle.row.id)
      tag.textContent = tagFor(handle.row.id, learn, locked)
    }
    session.on(mark)
    store.on((state, prev) => { if (state.learn !== prev.learn) mark() })
    mark()
  }
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

// A switch as one pill beside the others in a foot, pressed while it
// is on: for one that belongs with a row of choices, where the large
// switch would outweigh them.
export const flag = (handle, opts = {}) => {
  const button = h('button.kit-option', {
    type: 'button',
    onclick: () => handle.set(handle.value > 0.5 ? 0 : 1)
  }, handle.row.title)
  const show = () => button.setAttribute('aria-pressed', handle.value > 0.5)
  handle.on(show)
  show()
  return shell(handle, 'flag', opts, button)
}

// A list as a menu, its choice written as large as a number: for more
// choices than fit side by side.
export const menu = (handle, opts = {}) => {
  const { choices } = handle.row
  const last = Math.max(1, choices.length - 1)
  const at = () => Math.round(handle.value * last)
  const said = h('span.kit-chosen')
  const open = () => {
    const box = button.getBoundingClientRect()
    const list = popup(choices.map((name, i) => [
      name, i === at() ? 'chosen' : 'blank', '', () => handle.set(i / last)
    ]), [box.left, box.bottom + 6])
    list.querySelectorAll('button')[at()].focus()
  }
  const button = h('button.kit-chooser', {
    type: 'button',
    'aria-haspopup': 'menu',
    'aria-label': handle.row.title,
    onclick: open
  }, said, icon('chevron'))
  // The arrows step through the list without opening it.
  button.addEventListener('keydown', (event) => {
    const by = { ArrowUp: -1, ArrowLeft: -1, ArrowDown: 1, ArrowRight: 1 }
    if (!by[event.key]) return
    event.preventDefault()
    handle.set(clamp(at() + by[event.key], 0, last) / last)
  })
  const show = () => { said.textContent = choices[at()] }
  handle.on(show)
  show()
  return shell(handle, 'menu', opts,
    h('span.kit-title', {}, opts.title ?? handle.row.short), button)
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
    onclick: () => {
      delete el.dataset.clipped
      hold = { level: 0, at: 0 }
      show(false)
    }
  },
  h('span.kit-title', {}, opts.title ?? handle.row.short),
  out,
  h('div.kit-segments', { role: 'meter', 'aria-label': handle.row.title },
    segments))
  let hold = { level: 0, at: 0 }
  let said = handle.text
  let release = null
  // The bar rises at once and falls at a steady pace, so a level that
  // jumps about every reading can still be read.
  let shown = 0
  let last = 0
  let falling = null
  const bar = () => zoneFills(shown).forEach((fill, i) => {
    segments[i].style.setProperty('--fill', String(Number(fill.toFixed(3))))
  })
  const fall = (now) => {
    falling = null
    const seconds = last ? (now - last) / 1000 : 0
    last = now
    shown = fallen(shown, handle.value, seconds, fallFor(prefs.fall))
    bar()
    if (shown > handle.value + 1e-4) falling = requestAnimationFrame(fall)
    else last = 0
  }
  const show = (wait = true) => {
    const ms = holdFor(prefs.hold)
    const before = hold
    hold = held(hold, handle.value, performance.now(), ms)
    // The number is the peak that is held, in the host's own words
    // for it, not whatever the last reading happened to be.
    if (hold !== before) said = handle.text
    shown = Math.max(shown, handle.value)
    bar()
    if (shown > handle.value + 1e-4 && !falling) {
      falling = requestAnimationFrame(fall)
    }
    const [number, unit] = parts(said)
    out.replaceChildren(
      h('span.kit-number', {}, number),
      unit ? h('span.kit-unit', {}, ` ${unit}`) : ''
    )
    el.dataset.zone = zoneOf(hold.level)
    if (handle.value >= 0.999) el.dataset.clipped = 'true'
    clearTimeout(release)
    // Let go of the held peak even if no new reading comes.
    if (wait && ms && ms < Infinity) {
      release = setTimeout(() => show(false), ms + 20)
    }
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
// `pen.trail(meter)` is that meter's last few seconds, oldest first.
// `pen.signal` is the sound itself while audio runs, or null: `{ rate,
// input, output }`, the latest samples each way. `steady` finds where
// to start drawing one so it stands still, and `spectrum` turns one
// into bands.
// `opts.drag` makes the picture itself the control: `{ x, y }` are
// settings moved by dragging across and up, as one gesture.
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
    draw({
      ctx,
      width,
      height,
      touched: ui.touched(),
      trail: ui.trail,
      signal: ui.store.get().signal,
      ...inkOf(el)
    })
  }
  const ask = () => {
    if (queued) return
    queued = true
    requestAnimationFrame(paint)
  }
  ui.store.on(ask)
  if (opts.drag) draggable(el, ui, opts.drag)
  ;['resize', 'kit-theme', 'kit-touch', 'kit-density'].forEach((name) =>
    window.addEventListener(name, ask))
  requestAnimationFrame(paint)
  return el
}

// The picture as the control: across moves `x`, up moves `y`, from
// where each was when it was taken hold of. A double-click puts both
// back to their defaults.
const draggable = (el, ui, { x, y }) => {
  const moved = [x, y].filter(Boolean)
  let from = null
  el.dataset.drag = 'true'
  el.addEventListener('pointerdown', (event) => {
    if (event.button !== 0) return
    from = { px: event.clientX, py: event.clientY, x: x?.value, y: y?.value }
    el.setPointerCapture(event.pointerId)
    moved.forEach((handle) => handle.begin())
    ui.touch((y ?? x).row.key)
  })
  el.addEventListener('pointermove', (event) => {
    if (!from) return
    const box = el.getBoundingClientRect()
    const fine = event.shiftKey || prefs.fine === 'Always'
    x?.move(dragged(x.row, from.x, event.clientX - from.px, fine, box.width))
    y?.move(dragged(y.row, from.y, from.py - event.clientY, fine, box.height))
  })
  const stop = () => {
    if (!from) return
    from = null
    moved.forEach((handle) => handle.end())
    ui.touch(null)
  }
  el.addEventListener('pointerup', stop)
  el.addEventListener('pointercancel', stop)
  el.addEventListener('dblclick', () =>
    moved.forEach((handle) => handle.set(handle.row.default)))
}

// A tool in the header: an icon in a round target, named for a reader.
// `more` is what help adds about it.
const tool = (kind, name, label, onclick, more = '') => {
  const button = h(`button.kit-tool.kit-${kind}`, { type: 'button', onclick },
    icon(name))
  let told = label
  const say = (text) => {
    told = text
    button.setAttribute('aria-label', text)
  }
  say(label)
  tipped(button, (help) => ({ title: told, text: help ? more : '' }))
  return Object.assign(button, { say })
}

// Tools that belong together share a capsule.
const capsule = (...children) => h('div.kit-capsule', {}, children)

// The switch between the light and the dark theme.
export const themeSwitch = () => {
  const label = () => `Switch to the ${otherTheme(currentTheme())} theme`
  const button = tool('theme', 'theme', label(), switchTheme,
    'Light or dark, for every plugin of ours in this host.')
  window.addEventListener('kit-theme', () => button.say(label()))
  return button
}

// Roomy, Regular, Dense: each press steps to the next, and the icon
// shows which it is.
export const densitySwitch = () => {
  const label = () => {
    const now = currentDensity()
    return `Density: ${now}. Switch to ${nextDensity(now)}`
  }
  const button = tool('density', currentDensity(), label(), switchDensity,
    'Smaller numbers leave more room for the picture.')
  window.addEventListener('kit-density', () => {
    button.querySelector('path').setAttribute('d', ICONS[currentDensity()])
    button.say(label())
  })
  return button
}

// Step back and forward through the page's own changes, and flip
// between two sets of settings to hear which is better.
export const tools = (ui) => {
  const { session } = ui
  const undo = tool('undo', 'undo', 'Undo (Ctrl+Z)', session.undo,
    'Steps back through your own changes, one at a time.')
  const redo = tool('redo', 'redo', 'Redo (Ctrl+Shift+Z)', session.redo,
    'Steps forward again through what was undone.')
  const letters = ['A', 'B'].map((name) => h('span.kit-compare-slot', {}, name))
  const compare = h('button.kit-compare', {
    type: 'button',
    onclick: session.flip
  }, letters)
  tipped(compare, (help) => ({
    title: 'Compare A and B',
    text: help
      ? 'Keeps these settings and flips to the other set, to hear which ' +
        'is better. B starts as a copy of A.'
      : ''
  }))
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
  const settings = tool('settings', 'settings', 'Settings', () =>
    togglePanel('settings', settingsPanel(ui)),
  'Theme, density, how the meters move, and fine drag.')
  return [
    capsule(undo, redo),
    capsule(compare),
    capsule(densitySwitch(), themeSwitch(), settings, helpSwitch()),
    matcher(ui),
    power(ui)
  ]
}

// The head of the glass: the plugin's name, a note under it if the
// page has one to make (`opts.note`), and the tools, always in the
// same order so the hand learns them once.
export const header = (ui, opts = {}) => h('header.kit-header', {},
  h('div.kit-heading', {},
    tipped(h('h1.kit-name', {}, ui.name), () => ({
      title: ui.name, text: 'A Daily Audio Plugin: one of DAP\'s.'
    })),
    opts.note ? h('p.kit-note', {}, opts.note) : null),
  presetPill(ui),
  h('div.kit-tools', {}, tools(ui)))

// The foot of the sheet: whatever the page puts there, then what the
// control under the pointer is and does, or how the controls work
// when it is over none.
const HOW = 'Drag a number to change it. Shift for fine. Double-click it ' +
  'to type one, or its line for the default.'

export const foot = (ui, ...children) => {
  const what = h('b.kit-hint-what')
  const about = h('span.kit-hint-about', {}, HOW)
  const line = h('p.kit-hint', { 'aria-live': 'polite' }, what, about)
  // Its box is two lines and stays two lines: a longer text is drawn
  // smaller, so pointing at a control never moves anything.
  ui.hint = (title, text) => {
    what.textContent = title ?? ''
    about.textContent = title ? (text || '') : HOW
    fitText(line)
  }
  // What it says before anything is pointed at is fitted too, once
  // it is on the page and whenever the room for it changes.
  const fit = () => { if (line.isConnected) fitText(line) }
  requestAnimationFrame(fit)
  window.addEventListener('resize', fit)
  // A plugin with a valve or a transformer gets the way into its
  // window here, the same place in every one.
  return h('footer.kit-foot', {}, children, toneButtons(ui), line)
}

// Every level on the glass, every number on the sheet, every list in
// its foot: a whole page from the spec alone. A start, never the end:
// a plugin's page should show what the plugin does.
export const auto = (ui) => {
  const handles = Object.values(ui.p)
  // The bypass and level matching have their place in the header.
  const lists = handles.filter((handle) =>
    handle.row.kind === 'list' && !['bypass', 'match'].includes(handle.row.key))
  // A valve's and a transformer's own settings live in their windows.
  const windowed = (handle) => Object.values(TONES)
    .some((tone) => handle.row.key.startsWith(tone.prefix))
  const ranges = handles.filter((handle) =>
    handle.row.kind !== 'list' && !windowed(handle))
  const meters = Object.values(ui.m).map((handle) => meter(handle))
  return [
    glass(header(ui), stage(readouts(meters))),
    sheet(
      values(ranges.map((handle) => value(handle, { ui }))),
      foot(ui, lists.map((handle) => control(handle, { ui }))))
  ]
}

// ---- a value's menu, and its clipboard

let clip = null

const copyValue = (handle) => {
  clip = handle.text
  navigator.clipboard?.writeText(clip).catch(() => {})
}

// From the system's clipboard if the host lets the page read it, else
// what was last copied here.
const pasteValue = (handle) => {
  const take = (text) => {
    const norm = text ? parseValue(handle.row, text) : null
    if (norm !== null) handle.set(norm)
  }
  const read = navigator.clipboard?.readText?.()
  if (!read) return take(clip)
  read.then((text) => take(text || clip), () => take(clip))
}

// A menu at a point: [name, icon, key, run] a row. A press anywhere
// else, or Escape, puts it away.
const popup = (items, [x, y]) => {
  const app = document.getElementById('app')
  const close = () => {
    menu.remove()
    window.removeEventListener('pointerdown', away, true)
    window.removeEventListener('keydown', escape, true)
  }
  const away = (event) => { if (!menu.contains(event.target)) close() }
  const escape = (event) => {
    if (event.key !== 'Escape') return
    event.stopPropagation()
    close()
  }
  const menu = h('div.kit-pop', { role: 'menu' },
    items.filter(Boolean).map(([name, glyph, key, run]) => h('button', {
      type: 'button', role: 'menuitem', onclick: () => { close(); run() }
    }, icon(glyph), h('span', {}, name), h('kbd', {}, key))))
  app.append(menu)
  const zoom = Number(app.style.zoom) || 1
  const box = app.getBoundingClientRect()
  const left = (x - box.left) / zoom
  const top = (y - box.top) / zoom
  const mostLeft = app.clientWidth - menu.offsetWidth - 8
  const mostTop = app.clientHeight - menu.offsetHeight - 8
  menu.style.left = `${Math.min(left, mostLeft)}px`
  menu.style.top = `${Math.min(top, mostTop)}px`
  window.addEventListener('pointerdown', away, true)
  window.addEventListener('keydown', escape, true)
  menu.querySelector('button').focus()
  return menu
}

// What a right-click on a value offers.
const valueMenu = (el, handle, out, ui, at) => {
  const { row } = handle
  const locked = ui?.session.isLocked(handle.index)
  const learn = ui?.store.get().learn
  const cc = learn?.map[row.id]
  const waiting = learn?.waiting === row.id
  return popup([
    ['Type a value', 'type', 'Enter', () => out.edit()],
    [`Return to ${describe(row, row.default)}`, 'reset', 'Del',
      () => handle.set(row.default)],
    ['Copy', 'copy', 'Ctrl C', () => copyValue(handle)],
    ['Paste', 'paste', 'Ctrl V', () => pasteValue(handle)],
    ui && [locked ? 'Unlock' : 'Lock against presets', 'lock', '',
      () => ui.session.lock(handle.index, !locked)],
    // Only where the host side can hear a controller.
    learn && [waiting ? 'Stop waiting for a controller' : 'Learn a controller',
      'learn', '', () => post({ type: 'learn', id: waiting ? null : row.id })],
    learn && cc != null && [`Forget controller ${cc}`, 'close', '',
      () => post({ type: 'forget', id: row.id })],
    ui && linkItem(ui, row.title, [handle])
  ], at)
}

// ---- presets: the plugin's own, and the person's, kept by the page

const makePresets = (store, session, plugin, factory) => {
  const key = `kit-presets:${plugin}`
  const params = () => store.get().params
  const listeners = new Set()
  const tell = () => listeners.forEach((fn) => fn())
  const read = () => {
    try { return JSON.parse(window.localStorage.getItem(key)) ?? [] } catch {
      return []
    }
  }
  let own = read()
  const hosted = () => store.get().files != null
  const mine = () => (hosted() ? store.get().files : own)
  // Told again as they now are, without waiting for the host to say.
  const filed = (list) => store.receive({
    type: 'presets', list, dir: store.get().filesDir
  })
  let at = 'Init'
  let before = null
  const keep = () => {
    try { window.localStorage.setItem(key, JSON.stringify(own)) } catch {}
  }
  // Kept by key, so a preset outlives a setting added or moved.
  const byKey = (values) => Object.fromEntries(
    params().map((row, i) => [row.key, values[i]]))
  const fromKeys = (saved) => params().map((row) =>
    saved[row.key] ?? row.default)
  // The settings a preset names that this version does not have.
  const unknown = (values) => Object.keys(values ?? {})
    .filter((name) => !params().some((row) => row.key === name))
  const list = () => [
    { name: 'Init', by: 'Factory', values: params().map((row) => row.default) },
    ...factory.map((preset) => ({
      name: preset.name,
      by: 'Factory',
      values: presetValues(params(), preset.values),
      unknown: unknown(preset.values)
    })),
    ...mine().map((preset) => ({
      name: preset.name,
      by: 'Mine',
      values: fromKeys(preset.values),
      unknown: unknown(preset.values)
    }))
  ]
  let ask = null
  const go = (to) => presetMoves(store.get().values, to, session.locked)
    .forEach(([index, , after]) => session.put(index, after))
  const endPreview = () => {
    if (!before) return
    go(before)
    before = null
  }
  store.on((state, prev) => {
    // The first time the host says it keeps presets, those the page
    // had been keeping go to it, and the page keeps none.
    if (state.files && !prev.files && own.length) {
      own.filter((p) => !state.files.some((f) => f.name === p.name))
        .forEach((p) => post({ type: 'preset-save', ...p }))
      filed([...own, ...state.files.filter((f) =>
        !own.some((p) => p.name === f.name))])
      own = []
      keep()
    }
    if (state.values !== prev.values || state.files !== prev.files) tell()
  })
  return {
    on: (fn) => { listeners.add(fn); return () => listeners.delete(fn) },
    list,
    get name () { return at },
    // The settings are no longer what the named preset holds.
    get changed () {
      if (before) return false
      const preset = list().find((p) => p.name === at)
      const now = store.get().values
      return !preset || preset.values.some((v, i) =>
        Math.abs(v - now[i]) > 1e-9 && !session.isLocked(i))
    },
    // Hear a preset without keeping it; `endPreview` puts things back.
    preview: (preset) => {
      before = before ?? [...store.get().values]
      go(preset.values)
    },
    endPreview,
    // `fn(preset, go)` is asked before a preset with settings this
    // version does not know is loaded; `go` loads what fits.
    set ask (fn) { ask = fn },
    load: (preset) => {
      endPreview()
      const go = () => {
        session.apply(preset.values)
        at = preset.name
        tell()
      }
      if (preset.unknown?.length && ask) ask(preset, go)
      else go()
    },
    // Keeps what is set now under `name`; a factory name is left alone.
    save: (name) => {
      const taken = list().some((p) => p.name === name && p.by === 'Factory')
      const called = taken ? `${name} (mine)` : name
      const preset = { name: called, values: byKey(store.get().values) }
      const rest = mine().filter((p) => p.name !== called)
      if (hosted()) {
        post({ type: 'preset-save', ...preset })
        filed([...rest, preset])
      } else {
        own = [...rest, preset]
        keep()
      }
      at = called
      tell()
      return called
    },
    remove: (name) => {
      if (hosted()) {
        post({ type: 'preset-delete', name })
        filed(mine().filter((p) => p.name !== name))
      } else {
        own = own.filter((p) => p.name !== name)
        keep()
      }
      if (at === name) at = 'Init'
      tell()
    }
  }
}

// ---- linking: a setting that moves with the same one elsewhere

// Other instances by the names a person tells them apart by: several
// of one plugin are numbered in the order they were made.
export const linkNames = (peers) => {
  const count = (name) => peers.filter((peer) => peer.name === name).length
  return peers.reduce(({ seen, named }, peer) => {
    const nth = (seen[peer.name] ?? 0) + 1
    const name = count(peer.name) > 1 ? `${peer.name} ${nth}` : peer.name
    return {
      seen: { ...seen, [peer.name]: nth },
      named: [...named, { ...peer, label: name }]
    }
  }, { seen: {}, named: [] }).named
}

// How one other instance stands to these settings of ours: `can` are
// the settings it has too, by key, and `tied` those of them in the
// same group as ours. 'none' has no such setting, and cannot link.
export const linkState = (links, rows, peer) => {
  const can = rows.filter((row) => row.key in peer.keys)
  const tied = can.filter((row) => {
    const group = links.groups[row.id]
    return group && peer.keys[row.key] === group
  })
  const state = !can.length
    ? 'none'
    : tied.length === can.length ? 'all' : tied.length ? 'some' : 'off'
  return { can, tied, state }
}

// The instances any of these settings is linked to.
export const linkedTo = (links, rows) => (links?.peers ?? [])
  .filter((peer) => linkState(links, rows, peer).tied.length)

// The menu's way in, where the host side can link at all.
const linkItem = (ui, title, handles, back) => ui.store.get().links && [
  `Link ${title} with other instances`, 'link', '',
  () => togglePanel('link', linkPanel(ui, title, handles, back))
]

// The sheet turned over to who these settings are linked with: every
// other DAP instance there is, ticked if linked, and dimmed if it has
// no such setting. `back`, if given, is where Done returns to.
const linkPanel = (ui, title, handles, back) => () => {
  const rows = handles.map((handle) => handle.row)
  const list = h('div.kit-links', { role: 'group', 'aria-label': 'Instances' })
  const count = h('span.kit-about')
  const tie = (peer, can) => can.forEach((row) =>
    post({ type: 'tie', id: row.id, peer: peer.uid }))
  const untie = (peer, can) => can.forEach((row) =>
    post({ type: 'untie', id: row.id, peer: peer.uid }))
  const all = pill('All', false, () => {
    const links = ui.store.get().links
    links.peers.forEach((peer) => tie(peer, linkState(links, rows, peer).can))
  }, '.kit-link-all')
  const none = pill('None', false, () => {
    const links = ui.store.get().links
    links.peers.forEach((peer) =>
      untie(peer, linkState(links, rows, peer).tied))
    rows.forEach((row) => post({ type: 'untie', id: row.id }))
  }, '.kit-link-none')
  const draw = () => {
    const links = ui.store.get().links
    const peers = linkNames(links.peers)
    const tied = peers.filter((peer) =>
      linkState(links, rows, peer).state === 'all').length
    count.textContent = !peers.length
      ? 'No other DAP instance is open'
      : tied ? `${tied} linked` : 'Not linked'
    list.replaceChildren(...peers.map((peer) => {
      const { can, state } = linkState(links, rows, peer)
      const button = h('button.kit-option.kit-peer', {
        type: 'button',
        role: 'checkbox',
        'aria-checked': state === 'all' ? 'true' : state === 'some'
          ? 'mixed' : 'false',
        disabled: state === 'none',
        onclick: () => (state === 'all' ? untie(peer, can) : tie(peer, can))
      }, h('span.kit-check', {}, icon('chosen')), peer.label)
      return state === 'none'
        ? tipped(button, () => ({
            title: peer.label, text: `It has no ${title}.`
          }))
        : button
    }))
  }
  const off = ui.store.on((state, prev) => {
    if (!list.isConnected) return off?.()
    if (state.links !== prev.links) draw()
  })
  draw()
  return [
    h('div.kit-panel-bar', {},
      h('b.kit-panel-title', {}, `Link ${title}`),
      count,
      h('span.kit-space'),
      all,
      none,
      pill('Done', false, back ?? closePanel, '.kit-done')),
    list
  ]
}

// ---- panels: the sheet turned over to something else for a moment

const closePanel = () => {
  const sheetEl = document.querySelector('.kit-sheet[data-panel]')
  if (!sheetEl) return
  sheetEl.querySelector('.kit-panel')?.remove()
  sheetEl.style.minHeight = ''
  delete sheetEl.dataset.panel
  window.dispatchEvent(new Event('kit-panel'))
}

// Opens the panel `name` over the sheet, or closes it if it is open.
// The sheet keeps its height, so the glass does not jump.
const togglePanel = (name, build) => {
  const sheetEl = document.querySelector('.kit-sheet')
  if (!sheetEl) return
  const same = sheetEl.dataset.panel === name
  closePanel()
  if (same) return
  sheetEl.style.minHeight = `${sheetEl.offsetHeight}px`
  sheetEl.dataset.panel = name
  const panel = h('div.kit-panel', { 'data-panel': name }, build())
  panel.addEventListener('keydown', (event) => {
    if (event.key !== 'Escape') return
    event.stopPropagation()
    closePanel()
  })
  sheetEl.append(panel)
  window.dispatchEvent(new Event('kit-panel'))
  panel.querySelector('input, button')?.focus()
}

const pill = (name, on, onclick, kind = '') => h(`button.kit-option${kind}`, {
  type: 'button', 'aria-pressed': on, onclick
}, name)

// The preset browser: find one, hear it by pointing at it, press to
// keep it. Each one's bars are its first four numbers, so two can be
// told apart before either is tried.
const presetBrowser = (ui) => () => {
  const { presets } = ui
  const params = ui.store.get().params
  let filter = 'All'
  let naming = false
  let trying = null
  const find = h('input.kit-find', {
    type: 'text', placeholder: 'Find a preset', 'aria-label': 'Find a preset'
  })
  const bar = h('div.kit-panel-bar')
  const rows = h('div.kit-preset-list')
  const row = (preset) => h('div.kit-preset-row', {
    'data-on': String(preset.name === presets.name)
  },
  h('button.kit-preset-pick', {
    type: 'button',
    onclick: () => presets.load(preset),
    // Only a pointer that is really moving tries a preset: a list
    // redrawn under a still pointer must not change the sound.
    onpointermove: (event) => {
      if (!event.movementX && !event.movementY) return
      if (trying === preset.name) return
      trying = preset.name
      presets.preview(preset)
    }
  },
  h('span.kit-bars', { 'aria-hidden': 'true' },
    fingerprint(params, preset.values).map((value) =>
      h('i', { style: `--at: ${value}` }))),
  h('span.kit-preset-name', {}, preset.name),
  h('span.kit-preset-by', {}, preset.by)),
  preset.by === 'Mine'
    ? h('button.kit-preset-drop', {
      type: 'button',
      'aria-label': `Delete ${preset.name}`,
      onclick: () => presets.remove(preset.name)
    }, icon('close'))
    : null)
  const save = () => {
    const name = h('input.kit-find', {
      type: 'text', placeholder: 'Name it, then Enter', 'aria-label': 'Name'
    })
    name.addEventListener('keydown', (event) => {
      event.stopPropagation()
      if (event.key === 'Enter' && name.value.trim()) {
        presets.save(name.value.trim())
      }
      if (event.key === 'Enter' || event.key === 'Escape') {
        naming = false
        draw()
      }
    })
    return name
  }
  const draw = () => {
    const typed = find.value.trim().toLowerCase()
    const shown = presets.list()
      .filter((p) => filter === 'All' || p.by === 'Mine')
      .filter((p) => p.name.toLowerCase().includes(typed))
    const entry = naming ? save() : null
    bar.replaceChildren(
      find,
      ...['All', 'Mine'].map((name) => pill(name, filter === name, () => {
        filter = name
        draw()
      })),
      h('span.kit-space'),
      entry ?? pill('Save as new', false, () => { naming = true; draw() },
        '.kit-save'),
      pill('Done', false, closePanel, '.kit-done'))
    entry?.focus()
    rows.replaceChildren(...shown.map(row))
  }
  find.addEventListener('input', draw)
  find.addEventListener('keydown', (event) => {
    if (event.key !== 'Escape') event.stopPropagation()
  })
  rows.addEventListener('pointerleave', () => {
    trying = null
    presets.endPreview()
  })
  // Redrawn when the list or the one chosen changes, not as values move.
  const listed = () => `${presets.name}|${presets.list().map((p) => p.name)}`
  let last = listed()
  const stop = presets.on(() => {
    if (!rows.isConnected) return stop()
    if (listed() === last) return
    last = listed()
    draw()
  })
  // Put away with one still being tried: things go back as they were.
  const gone = () => {
    if (rows.isConnected) return
    presets.endPreview()
    window.removeEventListener('kit-panel', gone)
  }
  window.addEventListener('kit-panel', gone)
  draw()
  return [bar, rows]
}

// The name of what is loaded, beside the plugin's own: it opens the
// browser, and wears a dot once the settings are no longer the preset's.
const presetPill = (ui) => {
  const name = h('span.kit-preset-at')
  const button = h('button.kit-preset', {
    type: 'button',
    onclick: () => togglePanel('presets', presetBrowser(ui))
  }, name, icon('chevron'))
  const show = () => {
    name.textContent = ui.presets.name
    button.dataset.changed = String(ui.presets.changed)
    const edited = ui.presets.changed ? ', changed' : ''
    button.setAttribute('aria-label',
      `Preset: ${ui.presets.name}${edited}. Open the presets`)
  }
  ui.presets.on(show)
  show()
  return button
}

// Settings: kept for every plugin of ours, not saved with a song.
const settingsPanel = (ui) => () => {
  const body = h('div.kit-prefs')
  const saved = () => {
    try { return window.localStorage.getItem(THEME_KEY) } catch { return null }
  }
  const title = (text) => text[0].toUpperCase() + text.slice(1)
  const rows = () => [
    ['Theme', 'Follows the computer unless told', ['Light', 'Dark', 'Match'],
      saved() ? title(currentTheme()) : 'Match',
      (pick) => (pick === 'Match'
        ? matchTheme()
        : setTheme(pick.toLowerCase()))],
    ['Density', 'How much is shown at once', DENSITIES.map(title),
      title(currentDensity()), (pick) => setDensity(pick.toLowerCase())],
    ['Peak hold', 'How long a peak stays lit', PREFS.hold, prefs.hold,
      (pick) => setPref('hold', pick)],
    ['Motion', 'Whether anything eases', PREFS.motion, prefs.motion,
      (pick) => setPref('motion', pick)],
    ['Meter fall', 'How fast a bar comes down', PREFS.fall, prefs.fall,
      (pick) => setPref('fall', pick)],
    ['Fine drag', 'Hold Shift, or always', PREFS.fine, prefs.fine,
      (pick) => setPref('fine', pick)]
  ]
  const draw = () => body.replaceChildren(...rows().map(
    ([name, note, options, now, pick]) => h('div.kit-prefs-row', {
      'data-setting': name.toLowerCase().replace(' ', '-')
    },
    h('span.kit-prefs-name', {}, name, h('small', {}, note)),
    h('div.kit-options', { role: 'group', 'aria-label': name },
      options.map((option) => pill(option, option === now, () => {
        pick(option)
        draw()
      }))))))
  draw()
  return [
    h('div.kit-panel-bar', {},
      h('b.kit-panel-title', {}, 'Settings'),
      h('span.kit-about', {}, `${ui.name} ${ui.build}`),
      h('span.kit-space'),
      pill('Done', false, closePanel, '.kit-done')),
    body
  ]
}

// The plugin's own bypass, where the spec has a setting with the key
// `bypass`: filled when the plugin is on, hollow when it is bypassed.
const power = (ui) => {
  const handle = ui.p.bypass
  if (!handle) return null
  const button = tool('power', 'power', '', () =>
    handle.set(handle.value > 0.5 ? 0 : 1),
  'The same bypass as your host\'s own button. It fades, so it does not click.')
  const show = () => {
    const on = handle.value <= 0.5
    button.setAttribute('aria-pressed', on)
    button.say(on ? 'On. Press to bypass' : 'Bypassed. Press to turn on')
    document.documentElement.dataset.bypassed = String(!on)
  }
  handle.on(show)
  show()
  return linkable(ui, button, 'Bypass', [handle])
}

// A tool or a button that stands for settings: a right-click links
// them, and it says so while any of them is linked.
const linkable = (ui, button, title, handles) => {
  button.addEventListener('contextmenu', (event) => {
    event.preventDefault()
    const item = linkItem(ui, title, handles)
    if (item) popup([item], [event.clientX, event.clientY])
  })
  const rows = handles.map((handle) => handle.row)
  const show = () => button.toggleAttribute('data-linked',
    linkedTo(ui.store.get().links, rows).length > 0)
  ui.store.on((state, prev) => { if (state.links !== prev.links) show() })
  show()
  return button
}

// Level matching, where the spec has a setting with the key `match`:
// pressed in when the level going out is held to the level coming in.
const matcher = (ui) => {
  const handle = ui.p.match
  if (!handle) return null
  const button = tool('match', 'match', '', () =>
    handle.set(handle.value > 0.5 ? 0 : 1),
  'So a change is heard as tone, not as louder. Output still works on top.')
  const show = () => {
    const on = handle.value > 0.5
    button.setAttribute('aria-pressed', on)
    button.say(on
      ? 'Level matched to what comes in. Press to stop'
      : 'Match the level going out to the level coming in')
  }
  handle.on(show)
  show()
  return hinted(button, ui, handle, false)
}

// Help: while it is on, names are underlined and the foot of the
// sheet says more: what a setting does, its range and where it rests.
const helpSwitch = () => {
  const root = document.documentElement
  const button = tool('help', 'help', 'Help', () => {
    hideTip()
    root.dataset.help = String(root.dataset.help !== 'true')
    show()
  })
  const show = () => {
    const on = root.dataset.help === 'true'
    button.setAttribute('aria-pressed', on)
    button.say(on ? 'Help is on. Press to turn it off' : 'Help')
  }
  show()
  return button
}

// What help adds about a setting: its range and its resting place.
export const helpFor = (row) => {
  if (!row) return ''
  if (row.kind === 'list') return ` One of ${row.choices.join(', ')}.`
  if (row.kind !== 'range') return ''
  return ` From ${describe(row, 0)} to ${describe(row, 1)}. ` +
    `Rests at ${describe(row, row.default)}.`
}

// ---- settings in groups: a tab each, with a small chart of its values

// `list` is [{ name, handles }]. Every group's values show in its tab
// as bars, so nothing is hidden by being on another tab.
export const groups = (ui, list, opts = {}) => {
  const render = opts.render ?? ((handle) => control(handle, { ui }))
  let at = opts.start ?? 0
  const body = h('div.kit-values', { role: 'tabpanel' })
  const panels = list.map((group) => group.handles.map(render))
  const tabs = list.map((group, i) => {
    const charted = group.handles.slice(0, 6)
    const bars = charted.map(() => h('i'))
    const draw = () => charted.forEach((handle, n) =>
      bars[n].style.setProperty('--at', handle.value))
    charted.forEach((handle) => handle.on(draw))
    draw()
    return h('button.kit-tab', {
      type: 'button', role: 'tab', onclick: () => { at = i; show() }
    }, h('span.kit-bars', { 'aria-hidden': 'true' }, bars),
    h('span', {}, group.name))
  })
  const show = () => {
    tabs.forEach((tab, i) => tab.setAttribute('aria-selected', i === at))
    body.replaceChildren(...panels[at])
  }
  show()
  return [h('div.kit-tabs', { role: 'tablist' }, tabs), body]
}

// ---- the fader: used sparingly, where a level is ridden

// A line as wide as the meter's and a cap that carries the number.
// With `opts.meter`, a meter's handle, the line is that meter: the
// level plays on it in the same segments while the cap sets the gain.
export const fader = (handle, opts = {}) => {
  const cap = h('output.kit-fader-cap')
  const lit = opts.meter
    ? ZONES.map(({ name }) => h(`i.kit-segment.kit-zone-${name}`))
    : [h('i.kit-fader-fill')]
  const kind = opts.meter ? '.kit-fader-meter' : ''
  const track = h(`div.kit-fader-track${kind}`, {
    tabindex: 0,
    role: 'slider',
    'aria-orientation': 'vertical',
    'aria-label': handle.row.title
  }, h('div.kit-fader-line', {}, lit), cap)
  const el = shell(handle, 'fader', opts, track,
    h('span.kit-title', {}, opts.title ?? handle.row.short))
  const show = () => {
    track.style.setProperty('--at', handle.value)
    cap.textContent = parts(handle.text)[0]
    track.setAttribute('aria-valuetext', handle.text)
  }
  handle.on(show)
  show()
  if (opts.meter) {
    const level = () => zoneFills(opts.meter.value).forEach((fill, i) =>
      lit[i].style.setProperty('--fill', String(Number(fill.toFixed(3)))))
    opts.meter.on(level)
    level()
  }
  adjustable(track, handle, el, opts.ui, true)
  return el
}

// ---- two settings that move as one

// A handle on `lead` that takes `other` with it, by `tie`, for as long
// as `on()` says: both are one gesture for the host.
const tied = (lead, other, tie, on = () => true) => {
  let start = null
  const self = Object.assign(Object.create(lead), {
    begin: () => {
      lead.begin()
      if (!on()) return
      start = { lead: lead.value, other: other.value }
      other.begin()
    },
    move: (value) => {
      lead.move(value)
      if (start) other.move(tie(start, lead.value))
    },
    end: () => {
      lead.end()
      if (start) other.end()
      start = null
    },
    set: (value) => { self.begin(); self.move(value); self.end() }
  })
  return self
}

// Two settings held together: drag either and both go, keeping the
// gap between them. The link between them lets go and takes hold.
// `opts.mirror` moves them apart from a middle instead.
export const linked = (a, b, opts = {}) => {
  let on = opts.start ?? true
  const tie = TIES[opts.mirror ? 'mirrored' : 'linked']
  const button = h('button.kit-link', {
    type: 'button', onclick: () => { on = !on; show() }
  }, icon('link'))
  const show = () => {
    button.setAttribute('aria-pressed', on)
    button.setAttribute('aria-label', on
      ? `${a.row.title} and ${b.row.title} move together. Press to part them`
      : `Link ${a.row.title} and ${b.row.title}`)
  }
  show()
  return h(`div.kit-pair${opts.mirror ? '.kit-mirrored' : '.kit-linked'}`, {},
    value(tied(a, b, tie, () => on), opts),
    button,
    value(tied(b, a, tie, () => on), opts))
}

export const mirrored = (a, b, opts = {}) =>
  linked(a, b, { ...opts, mirror: true })

const readout = (handle, title) => {
  const out = h('output.kit-value')
  const show = () => {
    const [number, unit] = parts(handle.text)
    out.replaceChildren(h('span.kit-number', {}, number),
      unit ? h('span.kit-unit', {}, ` ${unit}`) : '')
  }
  handle.on(show)
  show()
  return h('div.kit-side', { 'data-key': handle.row.key },
    h('span.kit-title', {}, title ?? handle.row.short), out)
}

// One line shared by two: what one gains the other gives up. The
// place they meet is what is dragged.
export const opposed = (a, b, opts = {}) => {
  const bar = h('div.kit-oppose', {
    tabindex: 0,
    role: 'slider',
    'aria-label': `${a.row.title} against ${b.row.title}`
  }, h('i.kit-oppose-a'), h('i.kit-oppose-grip'), h('i.kit-oppose-b'))
  const el = shell(a, 'opposed', opts,
    h('div.kit-sides', {}, readout(a), readout(b)), bar)
  const show = () => {
    bar.style.setProperty('--at', a.value)
    bar.setAttribute('aria-valuetext', `${a.text}, ${b.text}`)
  }
  a.on(show)
  show()
  adjustable(bar, tied(a, b, TIES.opposed), el, opts.ui)
  return el
}

// Two ends of one thing: drag either end, or the stretch between to
// move both.
export const span = (lo, hi, opts = {}) => {
  const bar = h('div.kit-span', {
    role: 'group', 'aria-label': `${lo.row.title} to ${hi.row.title}`
  }, h('i.kit-span-fill'), h('i.kit-span-lo'), h('i.kit-span-hi'))
  const el = shell(lo, 'spanned', opts,
    h('div.kit-sides', {},
      readout(lo, opts.from ?? 'From'), readout(hi, opts.to ?? 'To')), bar)
  const show = () => {
    bar.style.setProperty('--lo', lo.value)
    bar.style.setProperty('--hi', hi.value)
  }
  lo.on(show)
  hi.on(show)
  show()
  let drag = null
  bar.addEventListener('pointerdown', (event) => {
    if (event.button !== 0) return
    const box = bar.getBoundingClientRect()
    const at = (event.clientX - box.left) / box.width
    const near = (v) => Math.abs(at - v) * box.width < 14
    const inside = at > lo.value && at < hi.value
    const which = near(lo.value)
      ? { handle: lo, most: () => hi.value, least: () => 0 }
      : near(hi.value)
        ? { handle: hi, most: () => 1, least: () => lo.value }
        : inside
          ? { handle: tied(lo, hi, TIES.linked), most: () => 1, least: () => 0 }
          : null
    if (!which) return
    drag = { ...which, x: event.clientX, from: which.handle.value, box }
    bar.setPointerCapture(event.pointerId)
    el.dataset.held = 'true'
    opts.ui?.touch(lo.row.key)
    drag.handle.begin()
  })
  bar.addEventListener('pointermove', (event) => {
    if (!drag) return
    const to = dragged(drag.handle.row, drag.from, event.clientX - drag.x,
      event.shiftKey, drag.box.width)
    drag.handle.move(clamp(to, drag.least(), drag.most()))
  })
  const stop = () => {
    if (!drag) return
    drag.handle.end()
    drag = null
    delete el.dataset.held
    opts.ui?.touch(null)
  }
  bar.addEventListener('pointerup', stop)
  bar.addEventListener('pointercancel', stop)
  return el
}

// ---- the pad: two settings moved by one point

// `x` runs across and `y` up. They are ordinary settings: their
// numbers move on the sheet as the point does.
export const pad = (x, y, opts = {}) => {
  const el = h('div.kit-pad', {
    'data-key': `${x.row.key}-${y.row.key}`,
    role: 'group',
    'aria-label': `${x.row.title} across, ${y.row.title} up`
  }, h('i.kit-pad-x'), h('i.kit-pad-y'), h('i.kit-pad-dot'))
  const show = () => {
    el.style.setProperty('--x', x.value)
    el.style.setProperty('--y', y.value)
  }
  x.on(show)
  y.on(show)
  show()
  let held = false
  const place = (event) => {
    const box = el.getBoundingClientRect()
    x.move(clamp((event.clientX - box.left) / box.width))
    y.move(clamp(1 - (event.clientY - box.top) / box.height))
  }
  el.addEventListener('pointerdown', (event) => {
    if (event.button !== 0) return
    held = true
    el.setPointerCapture(event.pointerId)
    el.dataset.held = 'true'
    opts.ui?.touch(x.row.key)
    x.begin()
    y.begin()
    place(event)
  })
  el.addEventListener('pointermove', (event) => { if (held) place(event) })
  const stop = () => {
    if (!held) return
    held = false
    x.end()
    y.end()
    delete el.dataset.held
    opts.ui?.touch(null)
  }
  el.addEventListener('pointerup', stop)
  el.addEventListener('pointercancel', stop)
  return el
}

// ---- a notice on the glass: nothing to show, or something wrong

// Says what happened and what to do about it. `opts.tone` is a short
// line in orange for when sound is affected; `opts.actions` are
// [{ name, run, main }], the main one filled in.
export const notice = (opts) => h('div.kit-notice', { role: 'status' },
  opts.tone ? h('span.kit-notice-tone', {}, opts.tone) : null,
  h('b.kit-notice-title', {}, opts.title),
  opts.text ? h('p', {}, opts.text) : null,
  opts.actions?.length
    ? h('div.kit-options', {}, opts.actions.map((action) =>
      pill(action.name, Boolean(action.main), action.run)))
    : null)

// Paired controls three across on a sheet, and faders side by side.
export const pairs = (...children) => h('div.kit-pairs', {}, children)

export const faders = (...children) => h('div.kit-faders', {}, children)

// ---- mid and side: hear either alone

// What stands beside the picture, one thing over another, at the foot
// of the glass.
export const column = (...children) => h('div.kit-column', {}, children)

// Both, Mid, Side: small pills on the glass, since it is for hearing
// with and not a part of the sound.
export const listen = (handle, opts = {}) => {
  const el = choice(handle, opts)
  el.classList.add('kit-listen')
  return el
}

// ---- the tube and the transformer: a window each, the same in every
// plugin that models one

// How far below its rest each stage cuts off, as kit_dsp::tube has
// them: Triode, Strapped, and a pentode, which has no such point.
const CUTOFF = [2.5, 6, null]

// The valve's curve as kit_dsp::tube_curve has it: the picture in the
// window and the sound are one curve, and a test on each side holds
// them to the same numbers.
export const tubeCurve = (x, grid, stage = 2) => {
  const far = CUTOFF[stage]
  const up = far ? Math.max(0, 1 + x / far) : 0
  const p = far ? far / 1.5 * (up * Math.sqrt(up) - 1) : x
  const soft = Math.tanh(p)
  if (p <= 0 || grid <= 0) return soft
  const ceiling = 1 - grid / 3
  return soft + (ceiling * Math.tanh(p / ceiling) - soft) * grid
}

// The core's curve, as kit_dsp::iron_curve has it: flux through a
// core pushed by `push`, from nickel (0) to steel (1).
export const ironCurve = (x, push, iron = 0) => {
  if (push < 1e-4) return x
  const u = x * push
  const nickel = u / Math.pow(1 + u ** 4, 0.25)
  return (nickel + (Math.tanh(u) - nickel) * iron) / push
}

// A stage that feeds `back` of itself round `curve`, with what goes
// in made up so a small sound keeps its level: where the two agree,
// found by halving, since more out always means less in.
const fedBack = (curve, back) => (x) => {
  if (back <= 0) return curve(x)
  const miss = (y) => curve(x * (1 + back) - back * y) - y
  const step = ([lo, hi]) => {
    const mid = (lo + hi) / 2
    return miss(mid) > 0 ? [mid, hi] : [lo, mid]
  }
  const [lo, hi] = Array.from({ length: 30 }).reduce(step, [-3, 3])
  return (lo + hi) / 2
}

// What each window is: its settings by key, its curve from them, and
// its characters: settings of the levers that stand for a kind of
// part. docs/tone.md says what each is after and how well founded.
export const TONES = {
  tube: {
    name: 'Tube',
    note: 'The valve itself',
    prefix: 'tube_',
    order: ['bias', 'grid', 'bloom', 'feedback', 'sag', 'glow'],
    reach: 3,
    curve: (p) => {
      const [bias, grid] = [p.tube_bias / 100, p.tube_grid / 100]
      const stage = p.tube_stage ?? 2
      const rest = tubeCurve(bias, grid, stage)
      return fedBack(
        (x) => tubeCurve(x + bias, grid, stage) - rest,
        (p.tube_feedback ?? 0) / 100 * 3)
    },
    characters: [
      {
        name: 'EF86 pentode, open',
        values: {
          tube_stage: 'Pentode', tube_bias: 10, tube_grid: 20,
          tube_bloom: 25, tube_feedback: 20, tube_sag: 15, tube_glow: 25
        }
      },
      {
        name: 'EF86 pentode, tight',
        values: {
          tube_stage: 'Pentode', tube_grid: 15, tube_bloom: 10,
          tube_feedback: 35, tube_sag: 5, tube_glow: 10
        }
      },
      {
        name: 'EF86 strapped',
        values: {
          tube_stage: 'Strapped', tube_grid: 5, tube_bloom: 15,
          tube_feedback: 50
        }
      },
      {
        name: '12AX7 stage',
        values: {
          tube_stage: 'Triode', tube_bias: 15, tube_grid: 50,
          tube_bloom: 45, tube_sag: 10, tube_glow: 10
        }
      },
      {
        name: 'Starved plate',
        values: {
          tube_stage: 'Triode', tube_bias: -60, tube_grid: 70,
          tube_bloom: 60, tube_sag: 40
        }
      },
      {
        name: 'Push-pull pair',
        values: {
          tube_stage: 'Pentode', tube_grid: 30, tube_bloom: 30,
          tube_feedback: 20, tube_sag: 60
        }
      }
    ]
  },
  transformer: {
    name: 'Transformer',
    note: 'The iron',
    prefix: 'iron_',
    order: ['drive', 'size', 'material', 'source', 'grip', 'top', 'ring'],
    reach: 1.2,
    curve: (p) => (x) => {
      const push = (p.iron_drive ?? 0) / 100 * 4
      const lean = (p.iron_grip ?? 0) / 100 * 0.35 / Math.max(1, push)
      const iron = (p.iron_material ?? 0) / 100
      return ironCurve(x + lean, push, iron) - ironCurve(lean, push, iron)
    },
    characters: [
      {
        name: 'Mic output, T14 kind',
        values: {
          iron_drive: 25, iron_size: 60, iron_material: 15,
          iron_source: 80, iron_top: 35, iron_ring: 10
        }
      },
      {
        name: 'Ouncer, O-12 kind',
        values: {
          iron_drive: 50, iron_size: 120, iron_material: 25,
          iron_source: 30, iron_top: 20, iron_ring: 30
        }
      },
      {
        name: 'Gapped output, LO1166 kind',
        values: {
          iron_drive: 25, iron_size: 30, iron_material: 80,
          iron_source: 5, iron_grip: 60, iron_top: 10, iron_ring: 20
        }
      },
      {
        name: 'Clean nickel input',
        values: {
          iron_drive: 10, iron_size: 20, iron_source: 10, iron_top: 5,
          iron_ring: 0
        }
      },
      {
        name: 'Big steel output',
        values: {
          iron_drive: 25, iron_size: 25, iron_material: 100,
          iron_top: 15, iron_ring: 40
        }
      }
    ]
  }
}

// The handles of a window's settings, in the window's own order
// whatever order the plugin lists them in.
const toneHandles = (ui, kind) => {
  const { prefix, order } = TONES[kind]
  const place = (handle) => {
    const at = order.indexOf(handle.row.key.slice(prefix.length))
    return at < 0 ? order.length : at
  }
  return Object.values(ui.p)
    .filter((handle) => handle.row.key.startsWith(prefix))
    .sort((a, b) => place(a) - place(b))
}

// The curve, drawn on the sheet in the sheet's own ink: what goes in
// across, what comes out up, a straight dashed line for a sound left
// alone, and purple while one of its settings is held.
const tonePicture = (ui, kind, handles) => {
  const tone = TONES[kind]
  const canvas = h('canvas')
  const el = h('div.kit-tone-curve', {
    role: 'img', 'aria-label': `The ${tone.name.toLowerCase()}'s curve`
  }, canvas)
  const paint = () => {
    const [width, height] = [canvas.clientWidth, canvas.clientHeight]
    if (!width || !height) return
    const app = document.getElementById('app')
    const density = (window.devicePixelRatio || 1) *
      (Number(app?.style.zoom) || 1)
    canvas.width = Math.round(width * density)
    canvas.height = Math.round(height * density)
    const ctx = canvas.getContext('2d')
    ctx.setTransform(density, 0, 0, density, 0, 0)
    const style = getComputedStyle(el)
    const ink = (name) => style.getPropertyValue(name).trim()
    const plain = Object.fromEntries(
      handles.map((handle) => [handle.row.key, handle.plain]))
    const curve = tone.curve(plain)
    const at = (x, y) => [
      (x / tone.reach + 1) / 2 * width,
      (1 - (y / tone.reach * 1.6 + 1) / 2) * height
    ]
    const line = (points) => {
      ctx.beginPath()
      points.forEach(([x, y], i) => (i ? ctx.lineTo(x, y) : ctx.moveTo(x, y)))
      ctx.stroke()
    }
    const xs = Array.from({ length: 97 }, (_, i) =>
      (i / 48 - 1) * tone.reach)
    ctx.lineCap = 'round'
    ctx.lineJoin = 'round'
    ctx.strokeStyle = ink('--track')
    ctx.lineWidth = 1
    line([at(-tone.reach, 0), at(tone.reach, 0)])
    line([at(0, -tone.reach), at(0, tone.reach)])
    ctx.strokeStyle = ink('--soft')
    ctx.setLineDash([2, 5])
    ctx.lineWidth = 1.5
    line(xs.map((x) => at(x, x)))
    ctx.setLineDash([])
    const held = handles.some((handle) => handle.row.key === ui.touched())
    ctx.strokeStyle = ink(held ? '--touch' : '--ink')
    ctx.lineWidth = 2.5
    line(xs.map((x) => at(x, curve(x))))
  }
  const ask = () => requestAnimationFrame(paint)
  handles.forEach((handle) => handle.on(ask))
  ;['kit-touch', 'kit-theme', 'resize'].forEach((name) =>
    window.addEventListener(name, () => { if (el.isConnected) ask() }))
  ask()
  return el
}

// The way from a window to who it is linked with: only where linking
// can be done, and marked while anything in the window is linked.
// Done, there, comes back here.
const linkButton = (ui, title, handles, back) => {
  if (!ui.store.get().links) return null
  const rows = handles.map((handle) => handle.row)
  const button = h('button.kit-tool.kit-link-open', {
    type: 'button',
    onclick: () => togglePanel('link', linkPanel(ui, title, handles, back))
  }, icon('link'))
  const names = () => linkNames(ui.store.get().links.peers)
    .filter((peer) => linkedTo(ui.store.get().links, rows)
      .some((tied) => tied.uid === peer.uid))
    .map((peer) => peer.label)
  const show = () => {
    const tied = names()
    button.toggleAttribute('data-linked', tied.length > 0)
    button.setAttribute('aria-label', tied.length
      ? `Linked with ${tied.join(', ')}`
      : `Link ${title} with other instances`)
  }
  const off = ui.store.on((state, prev) => {
    if (!button.isConnected) return off?.()
    if (state.links !== prev.links) show()
  })
  show()
  return tipped(button, () => ({
    title: button.getAttribute('aria-label'), text: ''
  }))
}

// A window's contents: its characters, its curve, its settings, and
// the way back to where they rest.
const toneWindow = (ui, kind) => () => {
  const tone = TONES[kind]
  const handles = toneHandles(ui, kind)
  // The window's settings as a character has them: what it does not
  // name is at its rest. Everything outside the window stays put.
  const as = (values) => {
    const state = ui.store.get()
    const all = presetValues(state.params, values)
    return handles.reduce((to, handle) => {
      to[handle.index] = all[handle.index]
      return to
    }, [...state.values])
  }
  const matches = (values) => {
    const want = as(values)
    return handles.every((handle) =>
      Math.abs(want[handle.index] - handle.value) < 1e-6)
  }
  const named = () => tone.characters.find(({ values }) => matches(values))
  const lists = handles.filter((handle) => handle.row.kind === 'list')
  const ranges = handles.filter((handle) => handle.row.kind !== 'list')

  const character = h('button.kit-option.kit-character', {
    type: 'button',
    'aria-haspopup': 'menu',
    onclick: () => {
      const box = character.getBoundingClientRect()
      const now = named()
      popup(tone.characters.map(({ name, values }) => [
        name, name === now?.name ? 'chosen' : 'blank', '',
        () => ui.session.apply(as(values))
      ]), [box.left, box.bottom + 6])
    }
  })
  const show = () => { character.textContent = named()?.name ?? 'Character' }
  handles.forEach((handle) => handle.on(show))
  show()

  return [
    h('div.kit-panel-bar', {},
      h('b.kit-panel-title', {}, tone.name),
      lists.map((handle) => choice(handle, { ui })),
      tipped(character, (help) => ({
        title: 'Character',
        text: help
          ? 'Settings of this window that stand for a kind of part. ' +
            'A start to refine from, not a copy.'
          : ''
      })),
      h('span.kit-space'),
      linkButton(ui, tone.name, handles,
        () => togglePanel(kind, toneWindow(ui, kind))),
      pill('At rest', false, () => ui.session.apply(as({})), '.kit-rest'),
      pill('Done', false, closePanel, '.kit-done')),
    h('div.kit-tone', { style: `--count: ${ranges.length}` },
      tonePicture(ui, kind, handles),
      h('div.kit-values', {}, ranges.map((handle) => value(handle, {
        ui, bipolar: handle.row.lo < 0
      }))))
  ]
}

// The pills that open the windows this plugin has, for the foot.
const toneButtons = (ui) => Object.keys(TONES)
  .filter((kind) => toneHandles(ui, kind).length)
  .map((kind) => {
    const tone = TONES[kind]
    const button = h(`button.kit-option.kit-open-${kind}`, {
      type: 'button',
      'aria-haspopup': 'dialog',
      onclick: () => togglePanel(kind, toneWindow(ui, kind))
    }, tone.name)
    // A dot, as on a preset: something in there is not at rest.
    const show = () => {
      const moved = toneHandles(ui, kind).some((handle) =>
        Math.abs(handle.value - handle.row.default) > 1e-9)
      button.dataset.moved = String(moved)
    }
    ui.store.on((state, prev) => { if (state.values !== prev.values) show() })
    show()
    linkable(ui, button, tone.name, toneHandles(ui, kind))
    return tipped(button, (help) => ({
      title: `${tone.name} window`,
      text: help ? `${tone.note}: the same settings in every plugin of ours ` +
        'that has one.' : ''
    }))
  })
