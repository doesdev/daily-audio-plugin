# 0007: Direction C, settled on a canvas before it was built

2026-11-25

The kit's page was rebuilt twice from a written brief and rejected
twice as dated and generic. The third time the design was drawn first,
on a design canvas, and nothing was built until it was approved.

Three directions were drawn for the same plugin. C was chosen: a dark
glass that holds the picture, a sheet of settings lying over its lower
edge, and the numbers themselves as the controls. It was then
developed board by board, and each board was agreed or corrected:

- purple only on what is being touched, on the sheet and on the glass
  at once;
- a level as six segments with space between, breaking at -24, -12,
  -6, -3 and 0 dB, with colour by zone and no gradient;
- three densities, so text can scale down and more can sit at the top
  level;
- a fader whose line is as wide as the meter's, and a variant whose
  line is the meter;
- gain reduction in ink, hanging from the top; the stereo field as a
  fan of the meter's segments;
- an icon set on a 20 grid with one 1.6 line;
- a flat keyboard, piano shaped, black keys square at the top, note
  names faint above it, sharps only;
- presets and a bypass in the header.

The boards were finally rendered and looked at, which found dark text
on dark in four of the dark boards and a dozen overflows and
misalignments. They were fixed on the canvas before the build.

What this costs: the canvas is now ahead of the kit. `docs/design.md`
lists what is drawn and not built. Those are built from the canvas
when a plugin needs them, not redesigned.

What it replaces: the look in 0006 (rule sliders, a turned knob, a
screen set into a deck). The rule that an editor meets the bar of the
best of its kind, and is built round a picture only it has, stands.

The canvas's source is kept in `design/canvas/`, one file a board, so
a later run can read what was agreed.
