// Template's editor. The screen shows the one thing this plugin does:
// the bend it puts in the signal. Under it, the settings that shape
// the bend; beside it, the way out.

import {
  mount, header, stage, screen, rail, deck, row, rule, knob, choice, meter,
  footer
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

// The screen has two views of the same thing. On the left, the bend:
// level in across, level out up, against the dashed line of a signal
// left alone, with the stretch the sound is using now lit. On the
// right, what that does to one cycle of a wave as loud as the sound
// coming in: dashed going in, solid coming out.
const bend = (ui) => {
  const { drive, shape, mix } = ui.p
  return (pen) => {
    const { ctx, width, height } = pen
    const pad = 18
    const size = height - pad * 2
    const wet = mix.plain / 100
    const out = (v) => v + (shaped(shape.plain, v * gain(drive.plain)) - v) * wet
    const live = Math.min(1, peakOf(ui.m.input.value))
    // With nothing coming in, the wave is drawn at a level to show the
    // shape by, in plain ink: it is not a reading.
    const reach = live > 0.001 ? live : 0.5
    const lit = live > 0.001 ? pen.accent : pen.ink
    const path = (points) => {
      ctx.beginPath()
      points.forEach(([px, py], i) => (i ? ctx.lineTo(px, py) : ctx.moveTo(px, py)))
      ctx.stroke()
    }
    const steps = (n) => Array.from({ length: n + 1 }, (_, i) => i / n)
    const frame = (left, w) => {
      ctx.lineWidth = 1
      ctx.strokeStyle = pen.line
      ctx.setLineDash([])
      ctx.strokeRect(left + 0.5, pad + 0.5, w - 1, size - 1)
      path([[left, pad + size / 2 + 0.5], [left + w, pad + size / 2 + 0.5]])
    }
    const y = (v) => pad + size / 2 - v * size / 2
    ctx.lineJoin = 'round'
    ctx.lineCap = 'round'

    // The bend.
    const x = (v) => pad + size / 2 + v * size / 2
    frame(pad, size)
    path([[x(0) + 0.5, pad], [x(0) + 0.5, pad + size]])
    ctx.strokeStyle = pen.soft
    ctx.setLineDash([2, 4])
    path([[x(-1), y(-1)], [x(1), y(1)]])
    ctx.setLineDash([])
    ctx.strokeStyle = pen.ink
    ctx.lineWidth = 1.25
    path(steps(240).map((t) => [x(t * 2 - 1), y(out(t * 2 - 1))]))
    if (live > 0.001) {
      ctx.strokeStyle = pen.accent
      ctx.lineWidth = 3
      path(steps(120).map((t) => {
        const v = (t * 2 - 1) * live
        return [x(v), y(out(v))]
      }))
    }

    // The wave.
    const left = pad * 2 + size
    const w = width - left - pad
    if (w < 80) return
    frame(left, w)
    const wave = (t) => reach * Math.sin(t * Math.PI * 2)
    const across = (t) => left + t * w
    ctx.strokeStyle = pen.soft
    ctx.lineWidth = 1
    ctx.setLineDash([2, 4])
    path(steps(240).map((t) => [across(t), y(wave(t))]))
    ctx.setLineDash([])
    ctx.strokeStyle = lit
    ctx.lineWidth = 2
    path(steps(480).map((t) => [across(t), y(out(wave(t)))]))
  }
}

mount((ui) => [
  header(ui),
  stage(
    screen(ui, bend(ui), {
      label: 'The bend it puts in the signal, and what that does to a wave'
    }),
    rail(
      row(meter(ui.m.input), meter(ui.m.level)),
      knob(ui.p.output, { ui })
    )),
  deck(
    rule(ui.p.drive, { ui }),
    choice(ui.p.shape, { ui }),
    rule(ui.p.tone, { ui }),
    rule(ui.p.mix, { ui })),
  footer(ui)
])
