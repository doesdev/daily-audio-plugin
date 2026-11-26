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
  mount, glass, header, stage, readouts, meter, picture, sheet, values,
  value, choice, foot
} from './kit/kit.js'

mount((ui) => [
  glass(
    header(ui, { note: 'Tape echo' }),  // name, undo, redo, A/B, density, theme
    stage(
      readouts(meter(ui.m.input), meter(ui.m.out)),
      picture(ui, (pen) => { /* the picture: see below */ }))),
  sheet(
    values(                             // four across
      value(ui.p.time, { ui }),
      value(ui.p.feedback, { ui }),
      value(ui.p.pan, { ui, bipolar: true })),
    foot(ui, choice(ui.p.mode, { ui }))) // and what the pointer is over
])
```

`ui.p` has a handle for each setting by key, `ui.m` for each meter.
Pass `{ ui }` to a control so the foot of the sheet can describe it
and the picture can tell when it is held.

- `value` is the house control: the number is the control. `bipolar`
  is for a setting that rests inside its range.
- `choice` is pills for up to four choices, `menu` for more, `toggle`
  for Off and On. `control(handle)` picks among them from the spec.
- `meter` is a level: a number over six segments. `fader` is for a
  level that is ridden; with `{ meter }` its line is that meter.
- `groups(ui, [{ name, handles }])` puts many settings on tabs, each
  tab a small chart of its values.
- `linked`, `mirrored`, `opposed` and `span` are for two settings
  that belong together, laid out with `pairs`. `pad(x, y)` moves two
  with one point. `notice` says on the glass that something is wrong.
- `mount(build, { presets })` gives the plugin its own presets:
  `[{ name, values: { key: value in its own terms } }]`. The header
  then has the preset browser; a person's own are kept by the page.
- `kit::bypass(id)` in `PARAMS` is the plugin's own bypass: the host
  is told it is the bypass, the runner fades to the sound coming in
  and back, and the header gets an On button. The plugin writes
  nothing for it.
- `kit::mix(id)` is the one Mix, and `kit::level_match(id)` with
  `kit_dsp::Match` holds the level going out to the level coming in:
  feed it the squares of what came in and what would go out, a frame
  at a time, and multiply by what it returns, before the output trim.
  The header shows it beside the bypass. Every plugin ships both, or
  says in its `benchmark.json` under `tools` why one makes no sense
  for it (an instrument has nothing coming in to match or to mix
  with). The template does both: copy how.
- `kit::listen(id)` and `kit::balance(id)` with `kit_dsp::MidSide`
  are how either half is heard alone and weighed against the other:
  multiply mid and side by what `next(balance, listen)` returns, as
  the last thing. On the page, `listen(ui.p.listen, { ui })` is the
  pills, and `column(...)` stacks them over the `readouts`.
- `kit_dsp::Tube` is the one valve and `kit_dsp::Transformer` the
  one transformer; `docs/tone.md` says what each lever stands for.
  Give the plugin `kit::tube_stage`, `tube_bias`, `tube_grid`,
  `tube_bloom`, `tube_feedback`, `tube_sag` and `tube_glow` (or
  `kit::iron_drive`, `iron_size`, `iron_material`, `iron_source`,
  `iron_grip`, `iron_top` and `iron_ring`), set the shape from them
  each block, and `foot` adds the button and the window, with its
  characters; `auto()` keeps those settings off the sheet. A plugin
  that moves the valve's resting point itself passes that to
  `Tube::next` as `swing`: the window's Bias adds to it. `tubeCurve`
  and `ironCurve` in `kit.js` are the same curves as `tube_curve` and
  `iron_curve`, held together by a test on each side with the same
  numbers: change one and change the other. A plugin that has any of
  the three says so in `benchmark.json`, as with Mix.
- A sidechain: `const KEY_IN: u32 = 2;` on the plugin gives it a
  second input in both formats, which the host routes another track
  to. `block.key(ch)` reads it (silence if nothing is routed; `None`
  if the plugin has none) and `block.keyed_io(ch)` gives it beside
  the input and output. `kit::key(id)` is the one Sidechain switch,
  off by default, since a host hands over an unrouted sidechain as
  silence: `flag(ui.p.key, { ui })` in the foot. In a test, the
  channels past the plugin's own input are its sidechain.
- Every setting can learn a MIDI controller from its menu, and what it
  learned is saved with the project. The plugin writes nothing for it.
  Every plugin takes MIDI in for this; notes still reach only one
  with `midi_in`. `docs/decisions/0008-reaching-across-vst3.md` says
  how it gets there.
- A person's own presets are files, one JSON file each, under
  `%APPDATA%\VENDOR\NAME\Presets`. The plugin writes nothing for
  that either.
- `mount(auto)` lays out a whole page from the spec alone: a start,
  never the end. `mount(build, { density: 'regular' })` starts a page
  with many settings smaller.
- A control the kit does not have is drawn on the design canvas first
  if it is not there already, then built from the kit's tokens. One a
  second plugin would want goes in `kit/ui/kit.js`, with a test in
  `kit/ui/test.js`.

**The picture is the main job.** `picture(ui, draw)` calls `draw(pen)`
whenever a setting or a meter moves or a setting is taken hold of.
`pen.ctx` is a canvas 2D context, `pen.width` and `pen.height` its
size, `pen.ink`, `pen.soft`, `pen.line` and `pen.accent` the glass's
colours, and `pen.touched` the key of the setting being dragged now,
or null. Draw what only this plugin has, from `ui.p.x.plain` and
`ui.m.y.value`, `pen.trail(ui.m.y)` (that meter's last four seconds)
and `pen.signal`, the sound itself while audio runs: `{ rate, input,
output }`, the last thousand samples each way, or null. `steady`
finds where to start drawing a wave so it stands still, and
`spectrum` turns samples into bands in dB. What goes in is dashed, what comes out in `pen.ink`, and
in `pen.accent` only the part that `pen.touched` shapes. The template
draws one cycle of a wave before and after its shaper, over the level
that went out. `picture(ui, draw, { drag: { x, y } })` makes the
picture the control: dragging across moves `x` and up moves `y`.

`ui/style.css` comes after the kit's and may set its tokens (at the
top of `kit/ui/kit.css`) or anything else. Every file the page needs
is listed in `ASSETS` in `src/lib.rs`; the screenshots are taken from
the page as the binary serves it, so a file left out shows up as a
broken scene.

A page must fill a window of any shape: the kit scales it and then
lays it out as wide and as tall as the window is. Build a picture to
draw at whatever size its canvas is given, not at the design's.

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
says `"theme": "light"`; show the editor in both. `steps` can click, slide, press and hold, hover and type
before the shot (`scripts/shots.js` lists them). `npm run shots --
NAME` writes `docs/images/NAME.png` for each.

`docs/manual.md` is the manual. `npm run docs -- NAME` fails unless it
names every setting in bold by its title, shows every picture, and has
no picture without a scene.

## 6. The demos

`examples/demo.rs` renders a few WAVs worth hearing, dry then through
the plugin. It runs as `cargo run -p NAME --example NAME-demo`: each
plugin's Cargo.toml names its examples after the plugin, because cargo
puts every package's examples in one folder and two called `demo`
would overwrite each other. `npm run new` does the naming. They ship in the release zip. Nobody has listened to a
plugin the daily run made until a person plays these, so pick the
settings that show what is new about it.

## What the kit does not do yet

- Sample-accurate automation: a setting takes one value a block.
- MIDI controllers into a VST3: notes arrive, CC and pitch bend do not
  (VST3 hands them over as parameters; wispidi's `controls.rs` shows
  how).
  This is the largest gap against the bar in `docs/design.md` and the
  next thing the kit needs.
- What `kit/benchmarks.json` marks `under`: the check prints the list.
- The parts of the design canvas that need a live signal or notes
  from the page: `docs/design.md` lists them.
- State beyond the settings: A/B and undo are forgotten when the
  editor closes.
- Sidechain inputs, more than one bus, mono-only or surround layouts.
- macOS and Linux editors. The audio side builds and is tested on
  Linux; the editor is Windows only.

Each is a kit change with its own tests when a plugin first needs it.
