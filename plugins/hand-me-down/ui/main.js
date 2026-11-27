// Hand-Me-Down's editor. The glass shows the family line: the last
// few notes along time, each one's own hit solid where it starts, its
// body after it, and over them an arc from each note back to the one
// its body came from. The upper half is the middle, the lower the
// sides, so a body handed into one half alone shows in that half.

import {
  mount, glass, header, stage, readouts, meter, picture, sheet, values,
  value, choice, menu, foot, h, column, listen, flag
} from './kit/kit.js'
import {
  RESTING, bodyAt, fall, heightOf, heirs, hitWeight, startsIn
} from './lineage.js'

// As src/lib.rs has them: the crossfade into a hit, in ms, and how
// many seconds the age meter takes to fill.
const CROSS_MS = 1
const AGE_S = 4

const gain = (db) => Math.pow(10, db / 20)

// What each setting shapes in the picture: holding one turns that
// part purple.
const HITS = ['hit', 'fade', 'hit_gain', 'threshold', 'gap']
const BODIES = ['body_gain', 'into', 'mix']
const ARCS = ['back']

// Into: which halves take the body handed down, the middle's first.
const TAKES = [[true, true], [true, false], [false, true]]

const draw = (ui) => (pen) => {
  const { ctx, width, height } = pen
  const p = ui.p
  const back = p.back.plain + 1
  const [midTakes, sideTakes] = TAKES[p.into.plain]
  const wet = p.mix.plain / 100
  const hitGain = gain(p.hit_gain.plain)
  const bodyGain = gain(p.body_gain.plain)
  const weightAt = (seconds) => hitWeight(seconds * 1000, CROSS_MS,
    p.hit.plain, p.fade.plain)

  // Live, the notes come from the meters' last few seconds: a column
  // a reading. At rest, a line of plucks is drawn to show the shape.
  const ages = pen.trail(ui.m.age)
  const found = startsIn(ages)
  const live = found.length >= 2
  const levels = pen.trail(ui.m.level)
  const handedTrail = pen.trail(ui.m.heir)
  const count = live ? ages.length : Math.max(60, Math.floor(width / 5))
  const wide = width / count
  const startsAt = live
    ? found
    : RESTING.starts.map((s) => s / RESTING.seconds * count)

  // A column's parts, from the centre out: the hit, then the body,
  // each as a level. Live, the meter has already put the whole on a
  // scale of decibels; at rest `heightOf` does.
  const columnAt = (i) => {
    if (live) {
      const seconds = ages[i] * AGE_S
      const level = levels[i] ?? 0
      const w = found[0] <= i ? weightAt(seconds) : 0
      const handed = handedTrail[i] > 0.5
      return { hit: w * level, body: (1 - w) * level, handed }
    }
    const t = i / count * RESTING.seconds
    const k = RESTING.starts.findLastIndex((s) => s <= t)
    if (k < 0) return { hit: 0, body: 0, handed: false }
    const age = t - RESTING.starts[k]
    const w = weightAt(age)
    const own = RESTING.levels[k] * fall(age)
    const body = bodyAt(RESTING, k, back, age)
    return {
      hit: w * own * hitGain,
      body: (1 - w) * bodyGain * (body.handed
        ? own + (body.level - own) * wet
        : own),
      own: (1 - w) * bodyGain * own,
      handed: body.handed && wet > 0
    }
  }

  const top = Math.round(height * 0.3)
  const mid = top + (height - top) / 2
  const tall = (height - top) / 2 - 6
  const held = (keys) => keys.includes(pen.touched)

  // The centre: the middle above, the sides below.
  ctx.strokeStyle = pen.line
  ctx.lineWidth = 1
  ctx.beginPath()
  ctx.moveTo(0, mid + 0.5)
  ctx.lineTo(width, mid + 0.5)
  ctx.stroke()

  // A column a reading, each half drawn out from the centre. A hit
  // is a solid slab, a body a comb of thin lines: handed down in ink,
  // a note's own body, where nothing was handed to that half, soft.
  for (let i = 0; i < count; i++) {
    const c = columnAt(i)
    const left = i * wide
    const thin = Math.max(1, Math.min(2, wide * 0.4))
    const half = (up, takes) => {
      const body = takes ? c.body : (c.own ?? c.body)
      const handed = takes && c.handed
      // The whole column on the scale, then shared between its parts.
      const whole = body + c.hit
      const reach = (live ? Math.min(1, whole) : heightOf(whole)) * tall
      const b = whole > 0 ? reach * body / whole : 0
      const hh = reach - b
      ctx.fillStyle = handed
        ? (held(BODIES) ? pen.accent : pen.ink)
        : pen.soft
      // The hit nearest the centre, so it shrinks inward as the body
      // takes over round it.
      const comb = left + (wide - thin) / 2
      ctx.fillRect(comb, up ? mid - reach : mid + 1 + hh, thin, b)
      ctx.fillStyle = held(HITS) ? pen.accent : pen.ink
      ctx.fillRect(left, up ? mid - hh : mid + 1, wide + 0.5, hh)
    }
    half(true, midTakes)
    half(false, sideTakes)
  }

  // Which half is which, and what ink and soft stand for.
  ctx.font = '13px "Bricolage Grotesque", sans-serif'
  ctx.textBaseline = 'middle'
  ctx.fillStyle = pen.soft
  ctx.textAlign = 'left'
  ctx.fillText('middle', 0, mid - 12)
  ctx.fillText('sides', 0, mid + 14)
  const key = (x, fill, words) => {
    ctx.fillStyle = fill
    ctx.fillRect(x, 6, 3, 12)
    ctx.fillStyle = pen.soft
    ctx.textAlign = 'left'
    ctx.fillText(words, x + 9, 12)
  }
  key(width - 196, pen.ink, 'handed down')
  key(width - 90, pen.soft, 'its own')

  // Over the notes, an arc from each to the note its body came from.
  const x = (at) => at * wide + wide / 2
  const line = heirs(startsAt.length, back)
  ctx.strokeStyle = held(ARCS) ? pen.accent : pen.soft
  ctx.fillStyle = held(ARCS) ? pen.accent : pen.ink
  ctx.lineWidth = 1.5
  startsAt.forEach((at, k) => {
    ctx.beginPath()
    ctx.arc(x(at), top - 4, 3, 0, Math.PI * 2)
    ctx.fill()
    const from = line[k]
    if (from == null || wet <= 0) return
    const [a, b] = [x(startsAt[from]), x(at)]
    // Never so high it reaches the key along the top.
    const rise = Math.max(4, Math.min(top - 32, 10 + (b - a) * 0.35))
    ctx.beginPath()
    ctx.moveTo(a, top - 4)
    ctx.quadraticCurveTo((a + b) / 2, top - 4 - rise * 2, b, top - 4)
    ctx.stroke()
  })
}

const PRESETS = [
  { name: 'One note late', values: { hit: 30, fade: 20 } },
  {
    name: 'Two back, slow to let go',
    values: { back: 'Two', hit: 60, fade: 80 }
  },
  { name: 'The old ring out wide', values: { into: 'Sides', hit: 25 } },
  {
    name: 'Drums pass it on',
    values: { hit: 12, fade: 8, threshold: 6, gap: 60 }
  }
]

const HOW_FAR = ['the note before', 'two notes back', 'three notes back',
  'four notes back']
const WHERE = ['', ', in the middle', ', in the sides']

// The note under the name says where each body comes from.
const lineage = (ui) => {
  const note = h('span')
  const show = () => {
    note.textContent = `Each body from ${HOW_FAR[ui.p.back.plain]}` +
      WHERE[ui.p.into.plain]
  }
  ui.p.back.on(show)
  ui.p.into.on(show)
  show()
  return note
}

mount((ui) => [
  glass(
    header(ui, { note: lineage(ui) }),
    stage(
      column(
        listen(ui.p.listen, { ui }),
        readouts(
          meter(ui.m.input, { title: 'In' }),
          meter(ui.m.level, { title: 'Out' }))),
      picture(ui, draw(ui), {
        label: 'The family line: each note\'s hit solid, its body after ' +
          'it, and an arc back to the note the body came from. Middle ' +
          'above, sides below. Drag across for Hit, up for Body level',
        drag: { x: ui.p.hit, y: ui.p.body_gain }
      }))),
  sheet(
    values(
      value(ui.p.threshold, { ui }),
      value(ui.p.gap, { ui }),
      value(ui.p.hit, { ui }),
      value(ui.p.fade, { ui }),
      value(ui.p.hit_gain, { ui, bipolar: true }),
      value(ui.p.body_gain, { ui, bipolar: true }),
      value(ui.p.mix, { ui }),
      value(ui.p.balance, { ui, bipolar: true }),
      value(ui.p.output, { ui, bipolar: true })),
    foot(ui, menu(ui.p.back, { ui }), choice(ui.p.into, { ui }),
      flag(ui.p.key, { ui })))
], { presets: PRESETS })
