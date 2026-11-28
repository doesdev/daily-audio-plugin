# Quality: oversampling, rendering, and defaults

Queued by the owner on 2026-11-28, to start once performance is
settled. Nothing here is built. This is what was asked for, how it is
meant to be done, and what has to be decided or found out first.

## What was asked for

1. **Oversampling that looks after itself.** State of the art, not
   seen by the person using the plugin, and spent only where it
   makes the sound better.
2. **Oversampling by hand**, set apart for playing and for rendering.
3. **Quality at any cost when rendering**, as an option.
4. **Any of these choices kept as the default**: for every instance
   of that plugin in the session, for that plugin from now on, and
   the same again for every DAP plugin.

## Why oversample at all

A valve or a transformer bends the sound. Bending makes harmonics,
and those above half the sample rate fold back down as tones that
were never played. Running the bend at several times the rate,
between two steep filters, puts the fold out of hearing. The cost is
processor time, a little delay, and whatever the filters themselves
do to the top of the band.

## The parts

### 1. The oversampler (kit/dsp)

- Rates of 2, 4, 8 and 16, made of half-band stages, so each
  doubling is one cheap filter.
- Two kinds of filter. One with next to no delay, for playing. One
  that keeps every frequency in step (linear phase), steeper, for
  rendering.
- The filter sums are laid out eight abreast and compiled twice
  (`docs/performance.md`), with a test that both give the same
  numbers.
- The delay it adds is told to the host, so tracks stay lined up.

### 2. Looking after itself ("Auto")

The rate a block needs depends on two things the plugin already
knows: how hard the bend is being driven, and how much of the sound
sits high up. A quiet bass through a gentle valve folds nothing
audible at one times; a bright cymbal driven hard needs eight.

- Each block, estimate the loudest folded tone from the drive and
  the top of the sound, and pick the lowest rate that keeps it under
  a floor well below hearing. The floor is a number in the kit with
  a test, not a taste.
- Changing rate must not be heard and must not move the sound in
  time: the delay told to the host stays fixed whatever rate is
  running, and one rate fades into the next.
- Where a bend has no memory, antiderivative anti-aliasing can do
  part of the job for almost nothing. The kit's valve and
  transformer both have memory, so this is for simpler shapers.
- Proof: a swept sine through each tone at each drive, with the
  folded tones measured against the floor, and the cost beside it.

### 3. By hand

One setting with two values, playing and rendering: Auto, Off, 2,
4, 8, 16. A host says when it is rendering (VST3's offline process
mode, CLAP's render extension); the kit reads neither yet.

### 4. Quality at any cost, rendering only

One switch. When the host renders: the highest rate, the linear
phase filters, and the tightest settings of whatever else a plugin
solves by steps (the valve's feedback is solved by Newton's method,
a few turns when playing). Two things to be plain about in the
editor: a render then differs slightly from what was heard while
mixing, and the delay differs between the two, which the host has to
be told as it changes.

### 5. Defaults, at four reaches

| Reach | Kept in |
| --- | --- |
| This instance | Its saved state, as now |
| Every instance of this plugin in the session | Told across by linking, which exists |
| This plugin from now on | A file in the DAP data folder, one a plugin |
| Every DAP plugin, now and from now on | One file in the same folder |

An instance either follows the default or holds a choice of its own.
The nearest wins: its own choice, then the plugin's default, then
DAP's, then what the kit ships. A project saved with a choice keeps
it; a project saved following the default hears the default change.
That last point is a decision, below.

Reading a file is never done on the audio thread: defaults are read
when an instance is made and when another instance says they
changed.

## To find out first

- What the leaders ship, written into `kit/benchmarks.json` with
  sources: which offer a separate rendering setting, which choose a
  rate by themselves and on what grounds, what filters they use and
  what delay they add. No "meets" until the swept sine says so.
- What a change of delay costs in each host when rendering starts.

## To decide (the owner's)

1. What playing defaults to. Proposed: Auto.
2. What rendering defaults to. Proposed: the same as playing, with
   quality at any cost off until chosen, so a render is what was
   heard unless asked otherwise.
3. Whether a saved project that follows the default should change
   when the default does. Proposed: yes, and the editor says so
   where the default is set.
4. Whether Auto may add delay. Proposed: a small fixed one, the same
   at every rate, so nothing ever shifts.

## Order of work

1. The oversampler, both filters, vectorised, with its measurements.
2. Rendering known to the kit in VST3 and CLAP.
3. The setting by hand, in the tube and transformer windows' shell.
4. Auto, with its proof.
5. Quality at any cost.
6. Defaults at the four reaches, and their place in the editor,
   decided on the design canvas before it is built.
