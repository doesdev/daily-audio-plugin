// Ironside's editor. The glass shows the two windings on one core. On
// the left, the core's curve, and how far up it the middle has filled
// it: the slope where it stands is how much the sides get through. On
// the right, what the sides' path does to each frequency: dashed where
// the corner sits with the core empty, solid where the middle has
// pushed it now, and the span it has climbed struck through.

import {
  mount, glass, header, stage, readouts, meter, picture, sheet, values,
  value, foot, h, column, listen, flag
} from './kit/kit.js'
import { MOST_FILL, cornerOf, fluxAt, ironSlope, keptDb } from './core.js'

// Holding one of these turns the sides' curve purple: it is what each
// of them shapes.
const SIDES = ['fill', 'corner', 'ceiling', 'recover', 'mix']
// And these the core's filled stretch.
const CORE = ['fill', 'iron_material', 'iron_size', 'iron_drive']

// The frequencies across the sides' panel, and the depth shown.
const LOW = 10
const HIGH = 20000
const DEEP = 27

// How far along the core's own scale the left panel shows.
const SPAN = 3

const fontOf = () =>
  getComputedStyle(document.body).fontFamily || 'sans-serif'

const draw = (ui) => (pen) => {
  const { ctx, width, height } = pen
  const p = ui.p
  const push = p.fill.plain / 100 * MOST_FILL
  const iron = p.iron_material.plain / 100
  const rest = p.corner.plain
  const ceiling = p.ceiling.plain * 1000
  const mix = p.mix.plain / 100
  const open = Math.max(0, 1 - ui.m.full.value)
  const now = cornerOf(rest, ceiling, open)
  const font = fontOf()
  ctx.lineJoin = 'round'
  ctx.lineCap = 'round'

  // ---- the core, on the left
  const side = Math.min(height - 44, width * 0.3)
  const cx = 14 + side / 2
  const cy = (height - 16) / 2
  const across = (u) => cx + u / SPAN * side / 2
  const up = (y) => cy - y / 1.4 * side / 2
  // The curve on its own scale: a push of one, so its shape is the
  // same at any Fill and only how far up it the middle gets moves.
  const shape = (u) => {
    const nickel = u / Math.pow(1 + u ** 4, 0.25)
    return nickel + (Math.tanh(u) - nickel) * iron
  }
  const along = (from, to, n = 80) => Array.from({ length: n + 1 },
    (_, i) => from + (to - from) * i / n)
  const trace = (us) => {
    ctx.beginPath()
    us.forEach((u, i) => (i ? ctx.lineTo : ctx.moveTo)
      .call(ctx, across(u), up(shape(u))))
    ctx.stroke()
  }
  ctx.strokeStyle = pen.line
  ctx.lineWidth = 1
  ctx.beginPath()
  ctx.moveTo(across(-SPAN), cy + 0.5)
  ctx.lineTo(across(SPAN), cy + 0.5)
  ctx.moveTo(cx + 0.5, up(1.4))
  ctx.lineTo(cx + 0.5, up(-1.4))
  ctx.stroke()
  ctx.lineWidth = 2
  trace(along(-SPAN, SPAN, 160))

  // How far up it the middle has filled it, both ways, and the slope
  // where it stands: steep is open, flat is shut.
  const reach = Math.min(SPAN, fluxAt(open, push, iron) * push)
  const coreHeld = CORE.includes(pen.touched)
  ctx.strokeStyle = coreHeld ? pen.accent : pen.ink
  ctx.lineWidth = 4
  if (reach > 0.01) trace(along(-reach, reach))
  const slope = push > 1e-4 ? ironSlope(reach / push, push, iron) : 1
  const tip = [across(reach), up(shape(reach))]
  const run = 0.9
  ctx.lineWidth = 1.5
  ctx.setLineDash([2, 4])
  ctx.beginPath()
  ctx.moveTo(across(reach - run), up(shape(reach) - slope * run))
  ctx.lineTo(across(reach + run), up(shape(reach) + slope * run))
  ctx.stroke()
  ctx.setLineDash([])
  ctx.fillStyle = coreHeld ? pen.accent : pen.ink
  ctx.beginPath()
  ctx.arc(tip[0], tip[1], 5, 0, Math.PI * 2)
  ctx.fill()

  // ---- the sides' path, on the right
  const left = 14 + side + 28
  const right = width - 12
  const top = 22
  const bottom = height - 26
  const x = (hz) => left + (right - left) *
    Math.log(hz / LOW) / Math.log(HIGH / LOW)
  const y = (db) => top + (bottom - top) * Math.min(1, -db / DEEP)
  const hzAt = (px) => LOW * Math.pow(HIGH / LOW, (px - left) / (right - left))
  const curve = (corner) => {
    ctx.beginPath()
    // Below the panel's depth the curve goes on out of sight; it is
    // not laid along the floor, which would read as a shelf.
    let seen = false
    for (let px = left; px <= right; px += 2) {
      const db = keptDb(hzAt(px), corner, mix)
      if (db < -DEEP) {
        seen = false
      } else {
        const yy = y(db)
        seen ? ctx.lineTo(px, yy) : ctx.moveTo(px, yy)
        seen = true
      }
    }
    ctx.stroke()
  }

  // A line at every decade, and the ceiling, faint.
  ctx.strokeStyle = pen.line
  ctx.lineWidth = 1
  ctx.beginPath()
  ;[10, 100, 1000, 10000].forEach((hz) => {
    ctx.moveTo(Math.round(x(hz)) + 0.5, top)
    ctx.lineTo(Math.round(x(hz)) + 0.5, bottom)
  })
  ;[0, -12, -24].forEach((db) => {
    ctx.moveTo(left, Math.round(y(db)) + 0.5)
    ctx.lineTo(right, Math.round(y(db)) + 0.5)
  })
  ctx.stroke()
  ctx.setLineDash([1, 3])
  ctx.beginPath()
  ctx.moveTo(Math.round(x(ceiling)) + 0.5, top)
  ctx.lineTo(Math.round(x(ceiling)) + 0.5, bottom)
  ctx.stroke()
  ctx.setLineDash([])

  // Where the corner has been over the last seconds.
  ctx.fillStyle = pen.line
  pen.trail(ui.m.full).forEach((full) => {
    const been = cornerOf(rest, ceiling, Math.max(0, 1 - full))
    ctx.fillRect(Math.round(x(been)) - 1, bottom + 6, 2, 8)
  })

  // What the middle has taken from the sides: struck through in
  // columns between the two curves.
  ctx.fillStyle = pen.soft
  for (let px = left + 2; px <= right; px += 5) {
    const hz = hzAt(px)
    const was = y(keptDb(hz, rest, mix))
    const is = y(keptDb(hz, now, mix))
    if (is - was > 1.5) ctx.fillRect(px, was + 1, 1.5, is - was - 2)
  }

  // At rest, dashed; now, solid.
  ctx.strokeStyle = pen.soft
  ctx.lineWidth = 1.5
  ctx.setLineDash([2, 5])
  curve(rest)
  ctx.setLineDash([])
  const sidesHeld = SIDES.includes(pen.touched)
  ctx.strokeStyle = sidesHeld ? pen.accent : pen.ink
  ctx.lineWidth = 2.5
  curve(now)

  // The corner now, marked where the sides are 3 dB down, and named
  // under it.
  const fill = sidesHeld ? pen.accent : pen.ink
  ctx.fillStyle = fill
  ctx.beginPath()
  ctx.arc(x(now), y(keptDb(now, now, mix)), 4, 0, Math.PI * 2)
  ctx.fill()
  const hz = Math.round(now)
  ctx.font = `500 13px ${font}`
  const said = hz >= 1000 ? `${(hz / 1000).toFixed(1)} kHz` : `${hz} Hz`
  const at = Math.min(right - ctx.measureText(said).width - 2,
    Math.max(left, x(now) + 10))
  ctx.fillText(said, at, y(keptDb(now, now, mix)) + 22)
  ctx.fillStyle = pen.soft
  ctx.font = `12px ${font}`
  ctx.fillText('Ceiling', Math.min(right - 44, x(ceiling) + 5), bottom - 6)

  // And how open the core is, under it.
  ctx.fillStyle = coreHeld ? pen.accent : pen.ink
  ctx.font = `500 13px ${font}`
  const shown = `${Math.round(open * 100)}% open`
  ctx.fillText(shown, cx - ctx.measureText(shown).width / 2,
    Math.min(height - 4, up(-1.4) + 18))
}

const PRESETS = [
  {
    name: 'Bass makes room',
    values: { fill: 50, corner: 30, ceiling: 1.2, recover: 5 }
  },
  {
    name: 'Grain on every swing',
    values: { fill: 80, corner: 40, ceiling: 3, recover: 0 }
  },
  {
    name: 'Slow steel',
    values: { fill: 40, recover: 180, iron_material: 100, iron_drive: 30 }
  },
  {
    name: 'Gentle thinning',
    values: { fill: 30, corner: 20, ceiling: 0.4, recover: 40, mix: 70 }
  }
]

// The note under the name says what fills the core.
const filling = (ui) => {
  const note = h('span')
  const show = () => {
    note.textContent = ui.p.key.plain
      ? 'The sidechain fills the core the sides are wound on'
      : 'The middle fills the core the sides are wound on'
  }
  ui.p.key.on(show)
  show()
  return note
}

mount((ui) => [
  glass(
    header(ui, { note: filling(ui) }),
    stage(
      column(
        listen(ui.p.listen, { ui }),
        readouts(
          meter(ui.m.input, { title: 'In' }),
          meter(ui.m.level, { title: 'Out' }))),
      picture(ui, draw(ui), {
        label: 'The core\'s curve, filled as far as the middle has filled ' +
          'it, and the sides\' low corner: dashed with the core empty, ' +
          'solid where the middle has pushed it. Drag across for ' +
          'Corner, up for Fill',
        drag: { x: ui.p.corner, y: ui.p.fill }
      }))),
  sheet(
    values(
      value(ui.p.fill, { ui }),
      value(ui.p.corner, { ui }),
      value(ui.p.ceiling, { ui }),
      value(ui.p.recover, { ui }),
      value(ui.p.mix, { ui }),
      value(ui.p.balance, { ui, bipolar: true }),
      value(ui.p.output, { ui, bipolar: true })),
    foot(ui, flag(ui.p.key, { ui })))
], { presets: PRESETS })
