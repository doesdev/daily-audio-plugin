# Writing a plugin on the kit

Start from `npm run new -- NAME "Display Name"` (add `--instrument`
for one that is played). Then work in this order; each step has
something that fails until it is done.

## 1. The spec

In `src/lib.rs`, `impl Plugin for Mine`:

```rust
const INFO: &'static Info = &Info {
    name: "Tape Bloom",
    role: Role::Effect,       // or Role::Instrument
    audio_in: 2,              // 0 for an instrument
    audio_out: 2,
    midi_in: false,           // true to be played, or to hear a keyboard
    midi_out: false,          // true to send MIDI
    size: (680, 250),         // the editor, in logical pixels
    min_size: (480, 250),
    clap_id: "...",           // made by `npm run new`: never change these
    vst3_processor: [...],
    vst3_controller: [...],
    ..
};

const PARAMS: &'static [Param] = &[
    range(0, "time", "Time", "Delay", (0.0, 500.0, 120.0), "ms")
        .step(5.0)            // lands on multiples of 5
        .decimals(0)          // shown with no decimal places
        .about("How far behind the echo runs"),
    range(1, "cutoff", "Cutoff", "Filter", (20.0, 20_000.0, 1000.0), "Hz")
        .skew(3.0),           // the low end gets more of the travel
    list(2, "mode", "Mode", "Delay", &["Tape", "Clean"], 0),
    toggle(3, "freeze", "Freeze", "Delay", false),
];

const METERS: &'static [Param] = &[
    meter(4, "out", "Output"),             // a level, shown in dB
    meter(5, "depth", "Depth").fraction(), // anything else, 0 to 1
];
```

- The first number is the id a host knows the setting by. Once a
  version has shipped it is never changed or reused: a new setting
  takes the next unused id. Order in the list is free.
- The key (`"time"`) is what the editor, the scenes and the docs call
  it. The title is what a host and the manual call it.
- Define `pub const TIME: usize = 0;` and so on for the indexes: that
  is how `process` reads them.

## 2. What it promises, failing

`tests/suite.rs` is already there and already holds the plugin to what
every plugin is held to (`kit/test/src/checks.rs`):

| Check | What fails it |
|-------|---------------|
| `spec_is_sound` | A reused id or key, a default out of range, text that does not read back |
| `output_is_finite` | NaN, infinity or a peak over 16, at any settings, on silence, a voice, noise or an impulse |
| `sounds_at_its_defaults` | Silence out when a voice goes in, or when it is played |
| `is_deterministic` | Two runs of the same input differing: seed anything random |
| `block_size_is_not_heard` | A bounce at blocks of 32 differing from one at 1024 |
| `never_allocates` | Any allocation or free in `process` |
| `is_within_budget` | Over 10% of a core on average, or a block over half its own length |
| `loads_in_every_format` | The VST3 or the CLAP describing itself, saving state or sounding unlike the plugin itself |

Write `tests/sound.rs` next: what this plugin does that no other does,
as assertions on rendered audio, before the code that makes them pass.
`kit_test::signals` has a voice, noise, a sine, an impulse and a played
phrase; `Runner::render` runs audio through the plugin.

## 3. The sound

```rust
fn prepare(&mut self, rate: f32, max_block: usize) {
    // Allocate here: delay lines, tables, voices.
}

fn process(&mut self, block: &mut Block) {
    let time = block.value(TIME);        // in its own terms: ms
    let mode = block.choice(MODE);       // a list's index
    let frozen = block.on(FREEZE);
    for ch in 0..block.outputs() {
        let (input, output) = block.io(ch);
        // ...
    }
    for i in 0..block.events.len() {
        let e = block.events[i];         // e.offset, e.data: MIDI in
    }
    block.send(0, [0x90, 60, 100]);      // MIDI out
    block.meter(OUT, level::to_fraction(peak));
}
```

- No allocation, lock or wait in `process`. The check counts.
- Input and output never alias, and a mono input answers for both
  channels: the kit owns the buffers.
- A setting holds one value a block. Ease anything that scales the
  signal (`kit_dsp::Smooth`), once a sample, or the block size will be
  heard and the check will say so.
- `latency()` and `tail()` say what `prepare` last set up.
- A block worth reusing (a filter, a pitch shifter, a voice allocator)
  goes in `kit/dsp` with its own tests, in its own commit, not in the
  plugin.

## 4. The editor

Read `docs/design.md` first: it is the bar an editor is held to, the
look, and what to avoid. `ui/main.js` composes the page:

```js
import {
  mount, header, stage, screen, rail, deck, row, rule, knob, choice,
  meter, footer
} from './kit/kit.js'

mount((ui) => [
  header(ui),                       // name, undo, redo, A/B, theme
  stage(
    screen(ui, (pen) => { /* the picture: see below */ }),
    rail(row(meter(ui.m.input), meter(ui.m.out)), knob(ui.p.output, { ui }))),
  deck(
    rule(ui.p.time, { ui }),
    choice(ui.p.mode, { ui }),
    rule(ui.p.feedback, { ui, bipolar: true })),
  footer(ui)                        // what the pointer is over
])
```

`ui.p` has a handle for each setting by key, `ui.m` for each meter.
Pass `{ ui }` to a control so the foot of the page can describe it.

- `rule` is the house control; `knob` is for the one or two settings
  that earn the room. `choice`, `menu`, `toggle` for lists. `control`
  picks for you.
- A control takes `{ tone: 'ok' | 'warn' | 'hot' | 'cool' }` where a
  colour says what it does, and `{ bipolar: true }` where its setting
  rests in the middle.
- A handle has `value` (0 to 1), `plain` (in its own terms), `text`,
  `row` (its spec), `on(fn)`, and `begin()`, `move(v)`, `end()` or
  `set(v)` for a gesture, so a control the kit lacks is a few lines.
  One a second plugin would want goes in `kit/ui/kit.js`, with a test
  in `kit/ui/test.js`.

**The screen is the main job.** `screen(ui, draw)` calls `draw(pen)`
whenever a setting or a meter moves. `pen.ctx` is a canvas context
sized in CSS pixels (`pen.width`, `pen.height`), and `pen.ink`,
`pen.soft`, `pen.line` and `pen.accent` are the screen's colours.
Draw what only this plugin has, from `ui.p.x.plain` and
`ui.m.y.value`. The template draws the bend it puts in the signal and
what that does to a wave as loud as the input.

`mount(auto)` lays out every setting with no more said. It has no
picture, so it is a place to start and never what ships.

`ui/style.css` comes after the kit's and may set its tokens (at the
top of `kit/ui/kit.css`) or anything else. Every file the page needs
is listed in `ASSETS` in `src/lib.rs`; the screenshots are taken from
the page as the binary serves it, so a file left out shows up as a
broken scene.

## 5. Scenes, pictures and the manual

`docs/scenes.json` lists the states of the editor worth showing:

```json
[
  { "name": "editor", "values": { "out": 0.7 } },
  { "name": "light", "theme": "light", "values": { "out": 0.7 } },
  { "name": "frozen", "values": { "freeze": "On", "time": 400 },
    "crop": { "sel": ".kit-group", "text": "Delay" } }
]
```

Values are by key, in the setting's own terms (a number, a choice's
name, a meter's fraction). A scene is shot in the dark theme unless it
says `"theme": "light"`; show the editor in both. `steps` can click, slide, drag, hover and type
before the shot (`scripts/shots.js` lists them). `npm run shots --
NAME` writes `docs/images/NAME.png` for each.

`docs/manual.md` is the manual. `npm run docs -- NAME` fails unless it
names every setting in bold by its title, shows every picture, and has
no picture without a scene.

## 6. The demos

`examples/demo.rs` renders a few WAVs worth hearing, dry then through
the plugin. They ship in the release zip. Nobody has listened to a
plugin the daily run made until a person plays these, so pick the
settings that show what is new about it.

## What the kit does not do yet

- Sample-accurate automation: a setting takes one value a block.
- MIDI controllers into a VST3: notes arrive, CC and pitch bend do not
  (VST3 hands them over as parameters; wispidi's `controls.rs` shows
  how).
- Anything from the audio thread to the editor but meters: no
  waveform, spectrum or history can be drawn from the real signal.
  This is the largest gap against the bar in `docs/design.md` and the
  next thing the kit needs.
- Presets.
- State beyond the settings: A/B and undo are forgotten when the
  editor closes.
- Sidechain inputs, more than one bus, mono-only or surround layouts.
- macOS and Linux editors. The audio side builds and is tested on
  Linux; the editor is Windows only.

Each is a kit change with its own tests when a plugin first needs it.
