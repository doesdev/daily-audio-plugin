# The look

Every editor in this repo starts from the same look, in `kit/ui`. A
plugin changes it only where its own subject asks for something the
kit does not have. This says what the look is, what each part of it
means, and what it avoids.

## What it is

Clean, modern and mostly one tone: a ground, ink a few steps from it,
and hairlines. Two themes, light and dark, switched by the half-full
disc at the top right and remembered for every editor the host shows.

| Token | Dark | Light | What it is for |
|-------|------|-------|----------------|
| `--ground` | `#1b1b1e` | `#ececee` | The page |
| `--raised` | `#26262a` | `#f7f7f8` | A knob's cap: the thing you take hold of |
| `--well` | `#303035` | `#dcdce0` | An empty track: a slider, a meter, a switch |
| `--line` | `#3a3a41` | `#cdcdd3` | Hairlines, and a knob's unlit ticks |
| `--ink` | `#eaeaed` | `#1d1d21` | Text you read: names, values |
| `--ink-soft` | `#9c9ca6` | `#62626c` | Text you glance at: a control's title |
| `--accent` | `#a690ff` | `#6a4ad4` | The value |
| `--accent-strong` | `#c6b8ff` | `#4f32b0` | The chosen option's text |
| `--accent-soft` | 16% of accent | 12% of accent | The chosen option's ground |

The greys are neutral with a breath of blue, so the purple sits in
them rather than on them. The off-white is cool, not cream.

## Purple means the value

Purple is the only accent and it says one thing: this is where the
setting is. A knob's lit ticks, a slider's fill, the chosen option, a
switch that is on, the focus ring. Nothing is purple for decoration.
If a thing is purple and is not a value or the focus, it is wrong.

## Other colours, only where they already mean something

| Token | Dark | Light | Means |
|-------|------|-------|-------|
| `--ok` | `#58c48a` | `#2f9461` | Fine |
| `--warn` | `#e3b24f` | `#c0851a` | Close to too much |
| `--hot` | `#f0716a` | `#cf4a42` | Too much |
| `--cool` | `#6cb4ec` | `#2e7fbf` | Cold, low, slow, the other of a pair |

A level meter is green to -12 dB, amber to -3 and red above, because
that is a language people already read. A meter of anything else is
purple: it is a value.

A control may take one of these as its `tone` (`knob(ui.p.heat,
{ tone: 'hot' })`) when the colour says what the control does: heat,
danger, a warm and a cool side of one process. Not to tell two knobs
apart, and not to make a page livelier. Most editors use none.

## Type

One family, Archivo, carried in the page (`kit/ui/fonts.css`) so an
editor looks the same in every host and in its screenshots. It has a
width axis, and the look uses it instead of a second typeface:

- the plugin's name: wide (112%), weight 650, 17px;
- a group's title: normal width, weight 600, 12px, sentence case;
- a control's title: a little narrow (92%), 11.5px, soft ink;
- a value: weight 550, 12px, tabular figures so it does not jitter as
  it changes.

## Shape

- No boxes. A group is its title and the space around it, with one
  hairline between it and the next. Controls are not on cards.
- A knob is a ring of 31 ticks, lit from where the setting rests to
  where it is, round a flat cap with a pointer. A setting that rests
  in the middle (`{ bipolar: true }`) lights from the middle out.
- Corners are round only where a thing is a shape you press: 7px on a
  row of options or a menu, a full round on a switch and a knob.
- No shadows and no gradients, but for the level meter's zones, which
  are hard-edged.
- Motion only answers a hand: a switch's thumb slides. Nothing moves
  on its own.

## What it avoids

These are the habits of generated interfaces. None of them is in the
kit and a plugin's own page should not bring them back:

- labels in capitals, or letter-spaced;
- a monospace face for numbers;
- every group in a rounded card with a border and a shadow;
- a gradient or a glow as decoration;
- a near-black ground with one bright accent, or a cream ground with
  a serif and a clay accent;
- an extra label above a thing that already has a name;
- colour used to make a page lively rather than to say something;
- emoji or arrows in labels.

## Making a plugin's editor its own

The house look is where an editor starts, and most of what makes one
editor different from the next should be what it shows: a control or a
picture that only this plugin could have, drawn from the same tokens.
Reach for a new colour last. If a plugin needs a control the kit does
not have, build it from `--ground`, `--line`, `--ink` and `--tone`, and
if a second plugin would want it, put it in the kit.

A plugin's `ui/style.css` may set any token. Changing `--accent` away
from purple is changing the house look, and needs a reason a person
would accept.

## Checking it

Every scene is shot in the dark theme unless it says `"theme":
"light"`. Each plugin shows its editor in both at least once. Look at
both before committing: contrast that works in one theme is the
commonest thing to break in the other.
