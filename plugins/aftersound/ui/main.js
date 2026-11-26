// Aftersound's editor. The glass shows a note's two decays as the
// settings make them: the middle, a line, falling fast as the bridge
// drinks what the strings do together; the sides, columns, rising out
// of nothing as the strings drift apart and ringing on. Where the sides
// take over is marked. Along the foot of the glass, the last seconds of
// what was played: the middle up, the sides down.

import {
  mount, glass, header, stage, readouts, meter, picture, sheet, values,
  value, foot, column, listen
} from './kit/kit.js'
import { decays, takesOver } from './foreseen.js'

// The settings each curve answers to: holding one turns it purple.
const SHAPES_MID = ['prompt']
const SHAPES_SIDE = ['after', 'mistune', 'strike']

// The quietest the picture shows, below the strike, in dB.
const FLOOR = -60

// How many points the curves are drawn from at most.
const POINTS = 360

const db = (x) => (x > 0 ? 20 * Math.log10(x) : -Infinity)

// How long the picture shows: long enough to see the sides take over
// and then ring, not so long the strike is a sliver.
const spanOf = ({ prompt, after }) =>
  Math.min(12, Math.max(2.5, after * 0.6, prompt * 2.5))

const seconds = (t) => (t < 1 ? `${t.toFixed(2)} s` : `${t.toFixed(1)} s`)

const draw = (ui) => (pen) => {
  const { ctx, width, height } = pen
  const settings = {
    prompt: ui.p.prompt.plain,
    after: ui.p.after.plain,
    mistune: ui.p.mistune.plain,
    strike: ui.p.strike.plain / 100
  }
  const span = spanOf(settings)
  const every = Math.max(1, Math.round(span * 220 / POINTS))
  const points = decays(settings.prompt, settings.after, settings.mistune,
    settings.strike, span, every)
  const over = takesOver(points)

  // The curves above, the last seconds played in a lane beneath.
  const lane = Math.min(54, height * 0.2)
  const [left, right, top] = [14, width - 14, 26]
  const bottom = height - lane - 40
  const x = (t) => left + (right - left) * t / span
  const y = (level) => {
    const d = Math.max(FLOOR, db(level))
    return top + (bottom - top) * d / FLOOR
  }
  ctx.lineJoin = 'round'
  ctx.lineCap = 'round'
  ctx.font = '13px "Bricolage Grotesque", sans-serif'
  ctx.textBaseline = 'alphabetic'

  // The floor, and a mark each second along it.
  ctx.strokeStyle = pen.line
  ctx.lineWidth = 1
  ctx.beginPath()
  ctx.moveTo(left, bottom + 0.5)
  ctx.lineTo(right, bottom + 0.5)
  ctx.stroke()
  const tick = span > 6 ? 2 : 1
  ctx.fillStyle = pen.soft
  Array.from({ length: Math.floor(span / tick) + 1 }, (_, i) => i * tick)
    .forEach((t) => {
      ctx.fillRect(Math.round(x(t)), bottom, 1, 5)
      ctx.textAlign = t === 0 ? 'left' : 'center'
      ctx.fillText(t === 0 ? 'strike' : `${t} s`, x(t), bottom + 19)
    })

  // The sides: a thin column every few pixels up from the floor.
  const sideHeld = SHAPES_SIDE.includes(pen.touched)
  const gap = 6
  ctx.fillStyle = sideHeld ? pen.accent : pen.soft
  for (let px = left; px <= right - 2; px += gap) {
    const t = (px - left) / (right - left) * span
    const at = points[Math.min(points.length - 1,
      Math.round(t / span * (points.length - 1)))]
    const tip = y(at.side)
    if (tip < bottom - 1) ctx.fillRect(px, tip, 2, bottom - tip)
  }

  // The middle: one line over them, until it is below the floor.
  const midHeld = SHAPES_MID.includes(pen.touched)
  ctx.strokeStyle = midHeld ? pen.accent : pen.ink
  ctx.lineWidth = 2.5
  ctx.beginPath()
  points.reduce((drawing, { t, mid }) => {
    const shown = db(mid) > FLOOR
    if (shown) (drawing ? ctx.lineTo : ctx.moveTo).call(ctx, x(t), y(mid))
    return shown
  }, false)
  ctx.stroke()

  // Where the sides take over: a dashed line down, and when.
  if (over !== null) {
    const at = points.find(({ t }) => t >= over)
    ctx.strokeStyle = pen.soft
    ctx.lineWidth = 1
    ctx.setLineDash([2, 4])
    ctx.beginPath()
    ctx.moveTo(Math.round(x(over)) + 0.5, top - 8)
    ctx.lineTo(Math.round(x(over)) + 0.5, bottom)
    ctx.stroke()
    ctx.setLineDash([])
    ctx.fillStyle = sideHeld || midHeld ? pen.accent : pen.ink
    ctx.beginPath()
    ctx.arc(x(over), y(at.side), 4.5, 0, Math.PI * 2)
    ctx.fill()
    ctx.fillStyle = pen.soft
    ctx.textAlign = x(over) > width * 0.7 ? 'right' : 'left'
    const shift = x(over) > width * 0.7 ? -8 : 8
    ctx.fillText(`the sides from ${seconds(over)}`, x(over) + shift, top - 4)
  } else {
    ctx.fillStyle = pen.soft
    ctx.textAlign = 'left'
    ctx.fillText('the sides never take over', left, top - 4)
  }

  // Which is which, in the corner: a stroke of line for the middle and
  // two columns for the sides.
  ctx.textAlign = 'right'
  ctx.fillStyle = sideHeld ? pen.accent : pen.ink
  ctx.fillText('sides', right, top + 14)
  const sidesAt = right - ctx.measureText('sides').width - 10
  ctx.fillStyle = sideHeld ? pen.accent : pen.soft
  ctx.fillRect(sidesAt - 2, top + 3, 2, 12)
  ctx.fillRect(sidesAt - 8, top + 6, 2, 9)
  const midAt = sidesAt - 22
  ctx.fillStyle = midHeld ? pen.accent : pen.ink
  ctx.fillText('middle', midAt, top + 14)
  const lineAt = midAt - ctx.measureText('middle').width - 10
  ctx.strokeStyle = midHeld ? pen.accent : pen.ink
  ctx.lineWidth = 2.5
  ctx.beginPath()
  ctx.moveTo(lineAt - 16, top + 9)
  ctx.lineTo(lineAt, top + 9)
  ctx.stroke()

  // The last seconds played: the middle up from the lane's centre,
  // the sides down from it, each by its level.
  const mids = pen.trail(ui.m.mid)
  const sides = pen.trail(ui.m.side)
  const centre = height - lane / 2 - 6
  const reach = lane / 2 - 2
  const count = Math.max(mids.length, 1)
  const wide = (right - left) / 120
  ctx.fillStyle = pen.line
  ctx.fillRect(left, Math.round(centre), right - left, 1)
  // Until something is played, what the lane is for.
  if (![...mids, ...sides].some((v) => v > 0)) {
    ctx.fillStyle = pen.soft
    ctx.textAlign = 'left'
    ctx.fillText('as played, the middle up and the sides down', left,
      centre - 5)
  }
  mids.forEach((m, i) => {
    const px = right - (count - i) * wide
    const s = sides[i] ?? 0
    ctx.fillStyle = pen.ink
    if (m > 0) ctx.fillRect(px, centre - m * reach, wide - 1.5, m * reach)
    ctx.fillStyle = pen.soft
    if (s > 0) ctx.fillRect(px, centre + 1, wide - 1.5, s * reach)
  })
}

const PRESETS = [
  {
    name: 'Grand unison',
    values: { prompt: 0.6, after: 7, mistune: 0.5, strike: 0, bright: 60 }
  },
  {
    name: 'Wide and slow',
    values: { prompt: 0.3, after: 25, mistune: 0.4, release: 1.5 }
  },
  {
    name: 'Beating pair',
    values: { prompt: 1.2, after: 9, mistune: 2.5, bright: 70 }
  },
  {
    name: 'Felt, off centre',
    values: { prompt: 0.8, mistune: 0.3, strike: 60, bright: 25 }
  }
]

mount((ui) => [
  glass(
    header(ui, { note: 'Two strings a note, one in each channel' }),
    stage(
      column(
        listen(ui.p.listen, { ui }),
        readouts(
          meter(ui.m.mid, { title: 'Mid' }),
          meter(ui.m.side, { title: 'Side' }))),
      picture(ui, draw(ui), {
        label: 'A note\'s two decays: the middle as a line, falling as ' +
          'the bridge takes what the strings do together; the sides as ' +
          'columns, rising as the strings drift apart and ringing on. ' +
          'Drag across for Mistune, up for Prompt',
        drag: { x: ui.p.mistune, y: ui.p.prompt }
      }))),
  sheet(
    values(
      value(ui.p.prompt, { ui }),
      value(ui.p.after, { ui }),
      value(ui.p.mistune, { ui }),
      value(ui.p.strike, { ui, bipolar: true }),
      value(ui.p.bright, { ui }),
      value(ui.p.release, { ui }),
      value(ui.p.balance, { ui, bipolar: true }),
      value(ui.p.output, { ui, bipolar: true })),
    foot(ui))
], { presets: PRESETS, density: 'regular' })
