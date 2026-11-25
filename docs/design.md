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

Before a plugin ships, its run names the two or three best plugins of
its kind in `NOVELTY.md` and says, for each of the seven, how this
one's editor compares. "Worse, and here is why that is acceptable
today" is an answer. Silence is not.

### What the kit gives every plugin

| | How |
|---|---|
| A picture | `screen(ui, draw)`: dark glass to draw on, redrawn as settings and meters move. What to draw is the plugin's job, and its main one. |
| Precision | Drag, Shift to go a tenth as fast, wheel and arrow keys by a step, double-click for the default, double-click a value (or Enter) to type one: `8k`, `1.2k Hz`, `-6`, the name of a choice. |
| Compare and recover | Undo and redo (the buttons, Ctrl+Z, Ctrl+Shift+Z) and an A/B switch, in every header. Each step goes to the host as a proper gesture. |
| Explains itself | The foot of the page says what the control under the pointer is and does, from the spec's `about`. Write an `about` for every setting. |
| Fits the screen | The page is drawn at the spec's size and scaled whole to whatever window the host gives it. |
| A character | The house look, below. |

### Where the kit is still under the bar

Be plain about these in `NOVELTY.md` until they are fixed:

- **No live signal on the screen.** Only settings and meters reach the
  page, so a picture can show what the plugin *would* do to a signal
  of the level coming in, not the signal itself: no waveform, no
  spectrum, no history of gain reduction. This is the biggest gap and
  the next thing the kit needs.
- **No presets.**
- **A and B and the undo history are forgotten** when the editor is
  closed; only the settings themselves are saved.
- **No modulation or MIDI learn**, which the leading synths have.

## Depth

The body has three depths and light comes from above:

- **Cut in**: slots, wells and the screen. Darker than the body, dark
  along the top edge, with a line of light under the bottom lip.
- **The body**: the page itself, one flat tone.
- **Standing off**: the deck and the rail, a rule's grip, a knob's
  cap, the chosen key. A touch lighter at the top than the bottom,
  bright along the top edge, a soft shadow below.

Everything you move stands off. Everything it moves in is cut in. That
is the whole grammar, and it is why depth here is not decoration: it
says what can be taken hold of. A thing held presses down (its shadow
tightens) and takes a purple edge.

The screen is dark glass in both themes. In the light theme it is the
one dark thing on the page, as a display is on a piece of equipment.

## Colour

Mostly one tone. The greys are neutral with a breath of blue; the
off-white is cool, not cream.

| Token | Dark | Light | What it is |
|-------|------|-------|------------|
| `--ground` | `#19191c` | `#dfdfe3` | The body |
| `--body-top`, `--body` | `#2b2b30`, `#232327` | `#fbfbfc`, `#efeff1` | A raised panel, top to bottom |
| `--cut` | `#131316` | `#d4d4d9` | Anything cut in |
| `--grip-top`, `--grip` | `#4a4a52`, `#36363c` | `#ffffff`, `#ececef` | Anything you take hold of |
| `--line` | `#3b3b42` | `#c4c4cb` | Printed marks: a rule's, a scale's |
| `--ink`, `--ink-soft` | `#ececef`, `#9d9da7` | `#1c1c20`, `#5f5f69` | Text you read; text you glance at |
| `--accent` | `#a690ff` | `#6a4ad4` | The value |
| `--screen` | `#101013` | `#101013` | The glass |

**Purple means the value, and nothing else.** A rule's fill and the
index on its grip, a knob's pointer, the chosen key's lettering, a
switch that is on, the lit part of a drawing on the screen, the focus
ring. If a thing is purple and is not where a setting is, or the
focus, it is wrong.

**Any other colour is news.** A level meter is ink, not a traffic
light: its column is the colour of text until the top 6 dB, which
warm, and a clip lights a small red lamp that stays lit until it is
clicked. A control may take a `tone` (`ok`, `warn`, `hot`, `cool`)
when the colour says what the control does. Most editors use none.

## The controls

Each was designed from its job, not from the usual picture of it.

- **Rule** (`rule`), the house control. A setting on one line: its
  title, a ruled slot with a grip, its value. The scale is printed on
  the panel, a small mark shows the default, and the travel from rest
  to the grip is filled. Titles and values line up down a deck, so a
  page of settings reads like a table. It is used instead of a bank
  of knobs because a rule shows where a setting is in its range at a
  glance, takes a fifth of the room, and drags the way it looks.
- **Knob** (`knob`), for the one or two settings that earn the room,
  usually the output. A plain turned cap on a skirt, three marks on
  its scale (least, default, most), and a purple pointer. No arc, no
  ring of lights.
- **Keys** (`choice`), a slot with the chosen key standing up.
- **Switch** (`toggle`), **menu** (`menu`).
- **Meter** (`meter`), a column of ink in a slot against a scale
  printed beside it, the peak held as a line for a moment, the level
  as a number beneath, and a clip lamp.
- **Screen** (`screen`), the picture.

### On pictures, meters and waves

These are where an editor most easily becomes every other editor. So:

- Draw the thing only this plugin has. Ask what its one idea looks
  like, not which standard display it could have.
- Do not draw a waveform or a spectrum because audio plugins have
  them. Draw one if it is how this plugin's effect is best seen, and
  then draw the *difference* the plugin makes, not just the signal.
- On the screen: thin ink for what is fixed, dashed soft ink for what
  the signal would have been, purple for what is happening now. No
  glow, no gradient fill under a curve, no neon.
- A meter is a reading, so it is ink. Do not colour it to liven the
  page.
- If the picture is computed from the settings rather than measured
  from the sound, it must still be true: the same arithmetic as the
  audio code, and tested if it is more than a line.

## Type

One family, Archivo, carried in the page so an editor looks the same
in every host and in its screenshots. Its width axis does the work a
second typeface would: the name is wide (114%, weight 680), a title a
little narrow (94%), values are weight 560 with tabular figures so
they do not jitter as they change. Sentence case throughout.

## What it avoids

The habits of generated interfaces. None is in the kit, and a
plugin's page should not bring one back:

- labels in capitals, or letter-spaced;
- a monospace face for numbers;
- every group in the same rounded card, whatever it is;
- a knob as an arc or a ring of lights; a meter as green, amber, red;
- a glow, or a gradient under a curve, as decoration;
- a near-black ground with one bright accent, or a cream ground with
  a serif and a clay accent;
- a label above a thing that already has a name;
- colour to make a page lively rather than to say something;
- emoji or arrows in labels.

## Making a plugin's editor its own

Spend the effort on the screen. The controls should look like the
house's; the picture should look like nothing else. A control the kit
does not have is built from the same three depths and the same
tokens, and goes in the kit if a second plugin would want it. A
plugin's `ui/style.css` may set any token; changing `--accent` away
from purple needs a reason a person would accept.

## Checking it

Every scene is shot in the dark theme unless it says `"theme":
"light"`. Each plugin shows its editor in both. Look at both before
committing: contrast that works in one is the commonest thing to
break in the other. Then put the picture beside a screenshot of the
best plugin of its kind and ask which you would rather use.
