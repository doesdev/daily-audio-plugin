// Template's editor. The glass shows the one thing this plugin does:
// what it does to a wave. Going in, dashed; coming out, solid. On the
// sheet under it, the numbers that shape that, and the way out.

import {
  mount, glass, header, stage, readouts, meter, picture, sheet, values,
  value, choice, foot, h, TRAIL
} from './kit/kit.js'

// One sample through the shaper, as src/lib.rs has it.
const shaped = (shape, x) => {
  if (shape === 0) return Math.tanh(x)
  if (shape === 1) return Math.max(-1, Math.min(1, x))
  const t = (((x + 1) % 4) + 4) % 4
  return t < 2 ? t - 1 : 3 - t
}

const gain = (db) => Math.pow(10, db / 20)

// A meter's fraction as the linear peak it stands for.
const peakOf = (fraction) => (fraction <= 0 ? 0 : gain(fraction * 60 - 60))

// The settings whose own shape is the one drawn: taking hold of one
// turns the wave purple, since the wave is then what is being touched.
const DRAWN = ['drive', 'mix']

// One cycle of a wave as loud as the sound coming in, before and
// after. With nothing coming in it is drawn at a level to show the
// shape by.
const wave = (ui) => (pen) => {
  const { drive, shape, mix } = ui.p
  const { ctx, width, height } = pen
  const wet = mix.plain / 100
  const out = (v) => v + (shaped(shape.plain, v * gain(drive.plain)) - v) * wet
  const live = Math.min(1, peakOf(ui.m.input.value))
  const reach = live > 0.001 ? live : 0.6
  const mid = height / 2
  const tall = height / 2 - 4
  const steps = (n) => Array.from({ length: n + 1 }, (_, i) => i / n)
  const path = (points) => {
    ctx.beginPath()
    points.forEach(([x, y], i) => (i ? ctx.lineTo(x, y) : ctx.moveTo(x, y)))
    ctx.stroke()
  }
  const sine = (t) => reach * Math.sin(t * Math.PI * 2)
  ctx.lineJoin = 'round'
  ctx.lineCap = 'round'

  // Behind the wave, the last few seconds of what went out: a column
  // a reading, the newest at the right.
  const past = pen.trail(ui.m.level)
  const wide = width / TRAIL
  ctx.fillStyle = pen.line
  past.forEach((level, i) => {
    const tall = level * height * 0.5
    const left = width - (past.length - i) * wide
    ctx.fillRect(left + 1, height - tall, Math.max(1, wide - 2), tall)
  })

  ctx.strokeStyle = pen.line
  ctx.lineWidth = 1
  path([[0, mid + 0.5], [width, mid + 0.5]])

  ctx.strokeStyle = pen.soft
  ctx.lineWidth = 1.5
  ctx.setLineDash([2, 5])
  path(steps(160).map((t) => [2 + t * (width - 4), mid - sine(t) * tall]))

  ctx.setLineDash([])
  ctx.strokeStyle = DRAWN.includes(pen.touched) ? pen.accent : pen.ink
  ctx.lineWidth = 2.5
  path(steps(480).map((t) => [2 + t * (width - 4), mid - out(sine(t)) * tall]))
}

// What it ships with: enough to show what the three bends are for.
const PRESETS = [
  { name: 'Warm edge', values: { drive: 9, tone: 4500, mix: 100 } },
  { name: 'Brick', values: { shape: 'Hard', drive: 24, tone: 9000 } },
  { name: 'Broken radio', values: { shape: 'Fold', drive: 32, tone: 2400 } }
]

const BENDS = ['Soft bend', 'Hard clip', 'Folded back']

// The note under the name says which bend is on.
const bend = (ui) => {
  const note = h('span')
  const show = () => { note.textContent = BENDS[ui.p.shape.plain] }
  ui.p.shape.on(show)
  show()
  return note
}

mount((ui) => [
  glass(
    header(ui, { note: bend(ui) }),
    stage(
      readouts(
        meter(ui.m.input, { title: 'In' }),
        meter(ui.m.level, { title: 'Out' })),
      picture(ui, wave(ui), {
        label: 'One cycle of a wave: going in dashed, coming out solid. ' +
          'Drag up and down for Drive',
        // The wave is the control: pull it up to drive it harder.
        drag: { y: ui.p.drive }
      }))),
  sheet(
    values(
      value(ui.p.drive, { ui }),
      value(ui.p.tone, { ui }),
      value(ui.p.mix, { ui }),
      value(ui.p.output, { ui, bipolar: true })),
    foot(ui, choice(ui.p.shape, { ui })))
], { presets: PRESETS })
