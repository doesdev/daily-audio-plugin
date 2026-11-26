// Undertow's editor. The glass shows the valve's curve and where on it
// the sides are: the middle's bass moves their resting point along
// the curve, and the stretch of curve they swing over is drawn round
// it. Dashed is where they would sit at rest; solid is where they are.

import {
  mount, glass, header, stage, readouts, meter, picture, sheet, values,
  value, choice, foot, h, column, listen, tubeCurve
} from './kit/kit.js'

// As src/lib.rs has it.
const REACH = 3

// How far either way of the centre the glass shows, on the curve's
// own scale: the farthest the bass can push, and room for the swing.
const SPAN = REACH + 0.4

// The settings that move what is drawn: holding one turns the riding
// stretch purple.
const DRAWN = ['push', 'follow', 'drive', 'steer', 'tube_bias', 'tube_grid']

// From the sound itself, the resting point the bias would give at
// each of the latest samples: the leaning half's low end, by Push.
// The editor is sent the middle, so this is only drawn live when the
// middle is the half that leans.
const biases = (ui, signal) => {
  const { push, follow } = ui.p
  const a = 1 - Math.exp(-2 * Math.PI * follow.plain / signal.rate)
  const gain = push.plain / 100 * REACH
  // One pole run over the samples, each step carrying the last.
  const follow1 = (low) => (x) => (low = low + (x - low) * a) * gain
  const out = signal.input.map(follow1(0))
  return out.slice(Math.floor(out.length / 2))
}

const draw = (ui) => (pen) => {
  const { ctx, width, height } = pen
  const mid = height / 2
  const tall = height / 2 - 18
  const across = (x) => width / 2 + x / SPAN * (width / 2 - 10)
  const up = (y) => mid - y * tall
  // The valve's own curve, as the tube window has shaped it, and how
  // far its Bias has moved where the carried half rests on it.
  const grid = ui.p.tube_grid.plain / 100
  const rest = ui.p.tube_bias.plain / 100
  const curve = (x) => tubeCurve(x, grid)
  const along = (from, to, n = 120) => Array.from({ length: n + 1 },
    (_, i) => from + (to - from) * i / n)
  const path = (xs) => {
    ctx.beginPath()
    xs.forEach((x, i) => (i ? ctx.lineTo : ctx.moveTo)
      .call(ctx, across(x), up(curve(x))))
    ctx.stroke()
  }
  ctx.lineJoin = 'round'
  ctx.lineCap = 'round'

  const midLeans = ui.p.steer.plain === 0
  const live = midLeans && pen.signal?.input.length ? pen.signal : null
  const wake = live ? biases(ui, live) : null
  const bias = rest + (wake
    ? wake[wake.length - 1]
    : ui.m.lean.value * REACH)
  const swing = Math.max(0.04, ui.m.swing.value * REACH)

  // The axes: the centre of the curve, where the sides rest unpushed.
  ctx.strokeStyle = pen.line
  ctx.lineWidth = 1
  ctx.beginPath()
  ctx.moveTo(0, mid + 0.5)
  ctx.lineTo(width, mid + 0.5)
  ctx.moveTo(across(0) + 0.5, 10)
  ctx.lineTo(across(0) + 0.5, height - 10)
  ctx.stroke()

  // Where the resting point has been: from the sound while it runs,
  // or from the lean meter's last seconds, which knows how far but
  // not which way, so it is drawn both ways.
  const been = wake
    ? wake.filter((_, i) => i % 6 === 0).map((x) => x + rest)
    : pen.trail(ui.m.lean)
      .flatMap((v) => [rest + v * REACH, rest - v * REACH])
  ctx.fillStyle = pen.soft
  been.forEach((x) => {
    ctx.fillRect(across(x) - 1, up(curve(x)) - 7, 2, 14)
  })

  // The whole curve, faint.
  ctx.strokeStyle = pen.line
  ctx.lineWidth = 2
  path(along(-SPAN, SPAN, 240))

  // At rest: the stretch the sides would swing over unpushed.
  ctx.strokeStyle = pen.soft
  ctx.lineWidth = 2
  ctx.setLineDash([2, 6])
  path(along(rest - swing, rest + swing))
  ctx.setLineDash([])

  // The push: from the centre along the axis to where the bass has
  // moved the resting point, and up from there to the curve.
  const held = DRAWN.includes(pen.touched)
  ctx.strokeStyle = held ? pen.accent : pen.ink
  ctx.lineWidth = 1.5
  ctx.beginPath()
  ctx.moveTo(across(rest), mid + 0.5)
  ctx.lineTo(across(bias), mid + 0.5)
  ctx.stroke()
  ctx.setLineDash([2, 4])
  ctx.beginPath()
  ctx.moveTo(across(bias) + 0.5, mid)
  ctx.lineTo(across(bias) + 0.5, up(curve(bias)))
  ctx.stroke()
  ctx.setLineDash([])

  // Now: the same swing, round where the bass has put it.
  ctx.strokeStyle = held ? pen.accent : pen.ink
  ctx.lineWidth = 4
  path(along(bias - swing, bias + swing))
  ctx.fillStyle = held ? pen.accent : pen.ink
  ctx.beginPath()
  ctx.arc(across(bias), up(curve(bias)), 6, 0, Math.PI * 2)
  ctx.fill()
}

const PRESETS = [
  { name: 'Room on the kick', values: { push: 80, follow: 90, drive: 9 } },
  {
    name: 'Sheen on the doubles',
    values: { push: 35, follow: 400, drive: 12, mix: 70 }
  },
  { name: 'Gentle undertow', values: { push: 40, follow: 160, drive: 4 } },
  {
    name: 'Sides on the middle',
    values: { steer: 'Side on mid', push: 70, follow: 200, drive: 6 }
  }
]

const NOTES = ['The middle leans on the sides', 'The sides lean on the middle']

// The note under the name says which half is leaning.
const leaning = (ui) => {
  const note = h('span')
  const show = () => { note.textContent = NOTES[ui.p.steer.plain] }
  ui.p.steer.on(show)
  show()
  return note
}

mount((ui) => [
  glass(
    header(ui, { note: leaning(ui) }),
    stage(
      column(
        listen(ui.p.listen, { ui }),
        readouts(
          meter(ui.m.input, { title: 'In' }),
          meter(ui.m.level, { title: 'Out' }))),
      picture(ui, draw(ui), {
        label: 'The valve\'s curve: dashed where the carried half would ' +
          'swing at rest, solid where the leaning half has pushed it. ' +
          'Drag up for Push, across for Drive',
        drag: { x: ui.p.drive, y: ui.p.push }
      }))),
  sheet(
    values(
      value(ui.p.push, { ui }),
      value(ui.p.follow, { ui }),
      value(ui.p.drive, { ui }),
      value(ui.p.mix, { ui }),
      value(ui.p.balance, { ui, bipolar: true }),
      value(ui.p.output, { ui, bipolar: true })),
    foot(ui, choice(ui.p.steer, { ui })))
], { presets: PRESETS })
