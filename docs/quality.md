# Quality: oversampling, rendering, and defaults

Asked for by the owner on 2026-11-28 and built the same day. This is
what there is, why it is the way it is, and where it is under.

## What was asked for

1. **Oversampling that looks after itself**: not seen by the person
   using the plugin, and spent only where it makes the sound better.
2. **Oversampling by hand**, set apart for playing and for rendering.
3. **Quality at any cost when rendering**, as an option.
4. **Any of these choices kept as the default**: for every instance
   of that plugin in the session, for that plugin from now on, and
   the same again for every DAP plugin.

The owner's two decisions: playing starts at Auto, and rendering
starts at Best.

## Why oversample at all

A valve or a transformer bends the sound. Bending makes harmonics,
and those above half the sample rate fold back down as tones that
were never played. Running the bend at several times the rate,
between two steep filters, puts the fold out of hearing. The cost is
processor time and a little delay.

## What a person sees

Two settings, in the Settings panel of any plugin that bends the
sound, under a line that says they are that plugin's own and saved
with the project:

- **Oversampling**, while playing: Auto, Off, 2x, 4x, 8x, 16x.
- **When rendering**, to a file: Best, As playing, Off, 2x to 16x.
- **Keep as default**: For this plugin, or For every DAP plugin.
  Pressing one does two things at once. Every instance open now that
  it reaches is set to match, and every instance made from now on
  starts there. A project saved before keeps what it was saved with.

## How it works

### The oversampler (`kit_dsp::Oversampled`)

- Rates of 2, 4, 8 and 16, each doubling one half-band filter. Every
  other tap of a half-band is nought, so a doubling costs one sum on
  the way up and one on the way down.
- Two sets of filters. When playing, they hold what they stop 80 dB
  down and the sound is 39 samples late. When rendering they hold
  it 130 dB down and it is 65 samples late.
- **The delay is the same whole number of samples at every rate**,
  Off included. A plugin tells its host that number once for playing
  and once for rendering, and moving between rates never moves the
  sound in time. Everything the bend is mixed with waits as long.
- **Changing rate is not heard.** The new rate runs beside the old
  until its filters are full, then the two are crossed over 32
  samples.
- A second signal that steers the bend (Undertow's bias) is carried
  up to the high rate and held back to meet the sound there.

### Auto (`kit_dsp::Need`)

A curve as smooth as a valve's makes harmonics that fall away by a
steady ratio from each to the next, and the ratio depends on how far
up the curve the sound reaches: gently driven each is a small part
of the last, driven hard they barely fall. The first harmonic that
can fold into hearing is the one that reaches past the running rate
less 20 kHz. So from two things the plugin already knows, how far up
the curve the sound reaches and how fast the sound going in moves,
the lowest rate that keeps what folds 100 dB down can be worked out,
sixteen samples at a time. That floor is `kit_dsp::FLOOR`.

- A bass driven hard runs at the plain rate: its harmonics run out
  long before they reach the fold. So does anything barely driven.
- A bright sound driven hard goes to 8 or 16.
- A rate is taken up at once and let go a step at a time, a quarter
  of a second after it stops being needed.
- A triode has a corner where it cuts off, and a sound that reaches
  near it is given the most there is.

### Rendering

A host says when it is rendering to a file: VST3 in its process
setup, CLAP through its render extension. The kit reads both and
tells the plugin (`Plugin::rendering`) before it asks how late the
plugin is, so the lateness told is the lateness for rendering.

At **Best** a render gets 16 times the rate, the steeper filters,
and the finest of whatever else a plugin solves by steps: a valve's
feedback is found in 8 turns of Newton's method, not 3.

### Defaults (`kit::defaults`, and linking's `take`)

Two small files beside the presets, one for the plugin and one for
every DAP plugin: a setting's key and its value on each line. A new
instance reads them when it is made, the plugin's own after the one
for all. Saying a default for all takes that setting out of every
plugin's own file, so the last thing said is what holds. Instances
open now are told over the same loopback channel linking uses.

## Where it is under

- **No setting has next to no delay.** The leaders' "zero latency"
  modes use filters that are not the same shape either side of the
  middle; the kit's are, so even Off is 39 samples late. Next: a
  minimum-phase set for playing.
- **A sudden brightening is caught a moment late.** Auto takes up a
  rate at once, but the new rate's filters take about two
  milliseconds to fill and cross in, and until then the old rate is
  heard. A render
  at Best is not affected. Next: look ahead by that much.
- **The top of the band at 44.1 kHz.** The filters leave everything
  up to 0.417 of the sample rate alone: 20 kHz at 48 kHz, 18.4 kHz
  at 44.1 kHz. Above that they start to fall, and what folds lands
  there too, out of hearing.
- **A hard grid, lifted and driven at the very top of the band**
  still leaves 80 dB down at 16 times, not 100: the curve has a join
  in it there that no rate here cures.
- **A render at Best is not sample for sample what was heard.** It
  is the same sound with less folded into it, and the editor's row
  says "Best". As playing is there for whoever wants the two alike.
- **Only Undertow uses it so far**: it is the one plugin with a
  valve. The transformer can be run through it and has a test, and
  no plugin has one yet.
- Not verified: any of this in a real host. That a host re-reads the
  lateness when it starts a render is the format's rule, and is what
  the kit's own test hosts do.
