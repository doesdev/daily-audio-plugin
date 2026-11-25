# The look, and the bar it is held to

Every editor in this repo starts from `kit/ui`. This says what the
bar is, what the kit gives every plugin toward it, what the look is
and what each part of it means, and what it avoids.

## The bar

The cornerstone rule: **a plugin's editor must meet or exceed what the
best makers in its field ship.** That does not mean looking like
theirs. It means asking, for the kind of plugin this is, what makes
the best ones a pleasure to use and to look at, and being at least
that good at each.

Looking across the makers people pay for, the same things recur:

1. **A picture that is the plugin.** The best editors are built round
   one live picture of what the plugin is doing to the sound, and it
   is theirs alone: an equaliser shows its curve over the spectrum, a
   compressor shows what it is taking away. Nobody could mistake it
   for another plugin's.
2. **Precision.** Fine adjustment, a value you can type, a default one
   gesture away, numbers that say what they are in.
3. **Compare and recover.** A and B, undo and redo, inside the plugin.
4. **It explains itself.** What a control does is one glance away, not
   in a manual.
5. **It fits the screen it is on.** The window resizes and everything
   scales with it.
6. **A character.** One look at a screenshot says who made it.
7. **Presets** that show what the plugin is for.

Before a plugin's editor is written, its run names at least two of
the best plugins of its kind in `benchmark.json` and says, for each
of the seven, how this one compares. "Under, and here is why that is
acceptable today" is an answer. Silence is not, and the check fails on
it. `docs/benchmarks.md` is the rule in full: it applies to every
interaction in the kit as well, one entry each in
`kit/benchmarks.json`.

### What the kit gives every plugin

| | How |
|---|---|
| A picture | `picture(ui, draw)`: the glass to draw on, redrawn as settings and meters move or are taken hold of. What to draw is the plugin's job, and its main one. |
| Precision | Drag a number, Shift to go a tenth as fast, wheel and arrow keys by a step, double-click its line for the default, double-click the number (or Enter) to type one: `8k`, `1.2k Hz`, `-6`, the name of a choice. While it is held, the ends of its range and its default are shown. |
| Compare and recover | Undo and redo (the buttons, Ctrl+Z, Ctrl+Shift+Z) and an A/B switch, in every header. Each step goes to the host as a proper gesture. |
| Explains itself | The foot of the sheet says what the control under the pointer is and does, from the spec's `about`. Write an `about` for every setting. |
| Fits the screen | The page is drawn at the spec's size and scaled whole to whatever window the host gives it. Density (Roomy, Regular, Dense) trades the size of the numbers for room on the glass. |
| A character | The house look, below. |

### Where the kit is still under the bar

Be plain about these in `NOVELTY.md` until they are fixed:

- **A and B and the undo history are forgotten** when the editor is
  closed; only the settings themselves are saved.
- **No modulation**, which the leading synths have.

## The design, and what of it is built

The look was settled on a design canvas before any of it was built
(`docs/decisions/0007-direction-c.md`). The canvas holds more than the
kit has yet. Built: the glass and the sheet, the number as the
control, pills, a menu, a switch, the segmented level, the header's
tools, density, both themes, the fader and the metering fader,
settings in groups with a small chart for each, linked, opposed,
mirrored and span controls, the pad, the preset browser, the settings
panel, a value's menu, help, and the notice for empty and error
states. `npm run gallery` draws them all, in both themes, into
`docs/images/kit/`.

Drawn and agreed but not built: the visual equaliser, gain reduction
over time and the stereo fan, which the kit can now feed (the sound
itself reaches the page, with a spectrum) and which wait for the
first plugin that needs them; the keyboard, which needs notes from
the page; and the envelope with modulation in blue, which needs a
modulation system. Each is built from the canvas with its own tests,
and not redesigned on the way.

## Depth

Two things, one over the other. A **glass**, near black in both
themes, holds the picture. A **sheet** in the theme's own tone holds
the settings and lies over the lower edge of the glass, with a shadow
under it. That overlap is the depth: no bevels, no inner glows, no
borders round groups. In the dark theme the sheet is the lighter of
two dark greys and the glass takes a hairline edge, so the two still
stand apart.

## Colour

Mostly one tone: ink on the sheet, off-white on the glass, and one
softer tone of each for what matters less. Colour is spent on three
things only.

- **Purple means: this is what you are touching.** A setting's name,
  number and line turn purple while it is dragged or typed, and what
  it shapes in the picture turns with it. Let go and it is ink again.
  Nothing is purple at rest but a switch that is on.
- **A level's colour is news.** Ink up to -24 dB, then green, yellow,
  orange and red, in segments with space between and no gradient. The
  number takes its segment's colour from yellow up.
- **Blue means a setting is being moved for you** (modulation). It is
  agreed on the canvas and reserved; the kit does not draw it yet.

The tokens are at the top of `kit/ui/kit.css`: the sheet's (`--ink`,
`--soft`, `--track`, `--touch`), the glass's (`--g-ink`, `--g-soft`,
`--g-line`, `--g-touch`) and a level's zones (`--z-green` and so on,
which differ on the light sheet so they hold their contrast).

## The controls

- **A value** (`value`): the number is the control. Its name, the
  number large with its unit small, and under it a 3 pixel line that
  shows where it sits in its range. A grip shows on the line under the
  pointer. A setting that rests inside its range, such as a trim or a
  pan, takes `bipolar` and fills from its default, with a mark there.
- **A choice of a few** (`choice`): pills, the chosen one filled in.
- **A choice of many** (`menu`): the choice written large over a line.
- **On or off** (`toggle`): a switch, and beside it what it is and
  which it is, in words.
- **A level** (`meter`): the number is the meter, with six segments
  under it breaking at -24, -12, -6, -3 and 0 dB. The top of the scale
  is stretched so the last 12 dB get half the bar. The last segment is
  the clip lamp and stays lit until clicked.

Anything pressed is at least 44 pixels (38 at Regular, 32 at Dense).

### On pictures, meters and waves

These are where an editor most easily turns into every other editor.
One way of drawing, whatever is drawn: what went in is dashed or dim,
what comes out is solid ink, what is being touched is purple. Columns
and blocks have gaps. Nothing glows and nothing is filled with a
gradient. Each plugin picks the one picture that shows its idea, and
may invent another in the same hand.

## Type

One family, Bricolage Grotesque, carried in the page so an editor
looks the same in every host and in the screenshots. Numbers are set
at 46 pixels (34, 26 by density), weight 500, tightly spaced; names
and notes at 13, in sentence case. No capitals for labels, no
monospace for numbers.

## What it avoids

The habits of generated interfaces. None is in the kit, and a
plugin's page should not bring one back:

- labels in capitals, or letter-spaced;
- a monospace face for numbers;
- every group in the same rounded card, whatever it is;
- a knob as an arc or a ring of lights; a meter as one bar fading
  from green to red;
- a glow, or a gradient under a curve, as decoration;
- a near-black ground with one bright accent, or a cream ground with
  a serif and a clay accent;
- a label above a thing that already has a name;
- colour to make a page lively rather than to say something;
- emoji or arrows in labels.

## Making a plugin's editor its own

Spend the effort on the picture. The controls should look like the
house's; the picture should look like nothing else. A control the kit
does not have is drawn on the canvas first if it is not there, built
from the same tokens, and goes in the kit if a second plugin would
want it. A plugin's `ui/style.css` may set any token; changing
`--touch` away from purple needs a reason a person would accept.

## Checking it

Every scene is shot in the dark theme unless it says `"theme":
"light"`. Each plugin shows its editor in both. Look at both before
committing: contrast that works in one is the commonest thing to
break in the other. Then put the picture beside a screenshot of the
best plugin of its kind and ask which you would rather use.
