// Template's editor: the shaper on the left, its colour in the middle,
// and what goes out on the right, with the level beside it.

import {
  mount, header, group, row, knob, choice, meter, h
} from './kit/kit.js'

mount((ui) => [
  header(ui),
  h('div.kit-body', {},
    group('Shape', row(knob(ui.p.drive), choice(ui.p.shape))),
    group('Colour', row(knob(ui.p.tone))),
    group('Output', row(
      knob(ui.p.mix),
      knob(ui.p.output),
      meter(ui.m.level, { vertical: true })
    )))
])
