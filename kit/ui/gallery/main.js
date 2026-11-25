// The kit's gallery: every control the kit has, on pages no plugin
// ships, over a made-up plugin. It is what the kit's own tests drive
// and what `npm run gallery` takes its pictures of. `?page=` picks one.

import {
  mount, glass, header, stage, readouts, meter, picture, sheet, values,
  value, choice, foot, groups, fader, faders, linked, mirrored, opposed,
  span, pairs, pad, notice
} from './kit/kit.js'

const page = new URLSearchParams(window.location.search).get('page') ?? 'main'

const PRESETS = [
  { name: 'Warm tape', values: { drive: 18, tone: 3200, mix: 80 } },
  { name: 'Broken radio', values: { drive: 34, tone: 1800, shape: 'Fold' } },
  { name: 'Glue', values: { drive: 6, mix: 40, output: 0 } }
]

// A flat line with a bump where Cutoff is: enough to show the glass
// answering a touch.
const curve = (ui) => (pen) => {
  const { ctx, width, height } = pen
  const at = ui.p.cutoff.value
  const lift = ui.p.resonance.value
  ctx.lineWidth = 2
  ctx.strokeStyle = pen.touched ? pen.accent : pen.ink
  ctx.beginPath()
  Array.from({ length: 121 }, (_, i) => i / 120).forEach((t, i) => {
    const bump = Math.exp(-Math.pow((t - at) * 9, 2)) * lift
    const fall = t > at ? Math.pow((t - at) / (1 - at + 0.01), 1.5) : 0
    const y = height * (0.45 - bump * 0.35 + fall * 0.5)
    if (i) ctx.lineTo(t * width, y)
    else ctx.moveTo(t * width, y)
  })
  ctx.stroke()
}

const pick = (ui, keys) => keys.split(' ').map((key) => ui.p[key])

const PAGES = {
  main: (ui) => [
    glass(
      header(ui, { note: 'Every control, for looking at' }),
      stage(
        readouts(meter(ui.m.input, { title: 'In' }), meter(ui.m.level)),
        picture(ui, curve(ui)))),
    sheet(
      values(
        value(ui.p.drive, { ui }),
        value(ui.p.tone, { ui }),
        value(ui.p.mix, { ui }),
        value(ui.p.output, { ui, bipolar: true })),
      foot(ui, choice(ui.p.shape, { ui })))
  ],
  groups: (ui) => [
    glass(header(ui), stage(picture(ui, curve(ui)))),
    sheet(
      groups(ui, [
        { name: 'Shape', handles: pick(ui, 'drive tone mix output') },
        { name: 'Filter', handles: pick(ui, 'cutoff resonance low high') },
        { name: 'Stereo', handles: pick(ui, 'mid side left right') }
      ], { start: 1 }),
      foot(ui))
  ],
  pairs: (ui) => [
    glass(
      header(ui),
      stage(
        pad(ui.p.cutoff, ui.p.resonance, { ui }),
        notice({
          tone: 'Sound passes unchanged',
          title: 'This preset was made by a newer version',
          text: 'Two of its settings are not known here.',
          actions: [
            { name: 'Load what fits', main: true, run: () => {} },
            { name: 'Cancel', run: () => {} }
          ]
        }),
        faders(
          fader(ui.p.send, { ui }),
          fader(ui.p.output, { ui, meter: ui.m.level, title: 'Out' })))),
    sheet(
      pairs(
        linked(ui.p.left, ui.p.right, { ui }),
        opposed(ui.p.mid, ui.p.side, { ui }),
        span(ui.p.low, ui.p.high, { ui })),
      foot(ui))
  ],
  mirror: (ui) => [
    glass(header(ui), stage(picture(ui, curve(ui)))),
    sheet(pairs(mirrored(ui.p.left, ui.p.right, { ui })), foot(ui))
  ]
}

mount(PAGES[page], { presets: PRESETS })
