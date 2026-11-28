# Backlog

Ideas waiting to be made, best first. None of these has been checked
for prior art: that is the first job of the run that picks one
(`docs/daily-run.md`, step 3). An idea leaves this list when it is
made (to the ledger) or dropped (to Dropped, below, with why).

## Waiting

Nothing that survived its search yet. The run of 2026-11-25 looked for
eight and made one (Undertow); the rest are under Dropped. The run of
2026-11-26 made the first of the ideas below (Aftersound), the run
of 2026-11-27 the next (Hand-Me-Down), and the run of 2026-11-28 the
last that had been searched (Ironside).

Nothing else that has been searched is waiting. The run of
2026-11-28 made the shared core (Ironside) as a saturable reactor;
the literal version is under Dropped.

A few directions the kit can already carry, to think from rather than
to build as written:

- An effect whose settings are driven by what it hears: the input's
  own level, pitch or brightness steering the process applied to it.
- Time as the thing processed: delays whose length follows the signal,
  or that read their line in an order other than forwards.
- An instrument with no oscillator in the usual sense: sound from a
  small rule run very fast.
- Two simple processes cross-coupled, each steering the other.

## What the listener reaches for

Said by the person these are made for, on 2026-11-25. These are not
ideas. They are levers: ways of working that they use a great deal and
enjoy seeing used inventively. An idea on well trodden ground can come
much closer to new by being built on one or two of them, and an idea
that could use one and does not should say why.

- **Mid and side.** Used a lot, and enjoyed most when it is used
  creatively: not only width and balance, but a process applied to one
  and not the other, the two treated oppositely, or one steering what
  is done to the other.
- **Split bands.** Two or more bands, each processed in its own way.
- **Transient and sustain.** The hit and the body of a sound split
  apart and processed separately, as a creative tool as much as a
  corrective one.
- **Transformer-inspired saturation.** A tone they like: the weight
  and the thickening of iron, with what it does to the low end.
- **Tube-inspired shimmer.** A tone they like: the bloom and the
  sheen of valves in the upper harmonics.
- **Instruments.** They want to see what can be done on the
  instrument side, not only effects. The same levers apply there: a
  voice whose mid and side, bands, or hit and body are made or shaped
  apart.

Two tools they expect on anything they make sense for, which the kit
now gives every plugin: a **Mix** of the effect against the sound that
came in, and **level matching**, which holds what goes out as loud as
what came in.

Three more follow from what a plugin is made of. Anything with **mid
and side** lets either be heard alone and one weighed against the
other. Anything that models a **valve** has the tube window, and
anything that models a **transformer** the transformer window: the
same four settings and the same picture in each plugin, so learning
one is learning all.

### What they hear in iron and glass

Built, as far as the evidence went: `docs/tone.md` has the levers,
the characters and how well founded each is. What they said, kept
here because it is the brief:

- **Transformers** they have owned in hardware and like most: the
  Haufe T14, the UTC O-12 and the Marinair LO1166.
- **Valves**: type, maker and wiring all matter. Their example is the
  EF86: a Telefunken adds a sheen up high, a Valvo is tight and
  punchy.
- Not a list of part numbers to copy: why two parts differ, and
  emulating that.

Still open: no character has been set against the hardware by ear;
the T14 character rests on the least; makers of one valve type are
not told apart beyond two EF86 settings; noise and microphony, the
screen's sag and oversampling are not built.

And a **sidechain** wherever one sound steers what is done to another:
the other track is as good a steerer as the sound's own other half.

These combine. A split is a place to put a tone; a tone is a reason
for a split. The kit's paired controls (linked, mirrored, opposed, a
span) and the pad were drawn with mid and side and band edges in mind.

None of this lowers the bar in step 3. A lever is not novelty by
itself: mid and side saturation and multiband transient shapers exist.
Look for the idea as hard as for any other, and say in `NOVELTY.md`
what the lever is doing that it has not done before.

When a run makes something on one of these, note it under "What has
gone well" once a person has listened, so the levers that pay off are
known.

## What has gone well

Nothing yet. After a person has listened to a few releases, note here
what kinds of idea they rated: it is the best guide to the next one.

## Dropped

| Idea | Why | Link |
|------|-----|------|
| A delay timed by the gap between the player's own last two hits, followed continuously, no tempo | RhythmEcho (beta, 2026) sets its delay from the detected beat period, updated on each detected beat | https://way.net/rhythmecho/RhythmEchoManual.pdf |
| Mid and side trade envelopes: each takes the other's dynamics | Imposing one signal's envelope on another is done by envelope imposers; in mid and side it is routing, not new | https://www.kvraudio.com/product/amplitude_imposer_by_ndc_plugs |
| Harmonics made from the mid put only in the side, so they cancel in mono | Panoramic's Steal derives harmonics from the mid and puts them in the stereo image | https://panoramalabs.com.au/panoramic/ |
| A delay whose repeats turn through the stereo field, an angle each time | Stereo rotation in a delay's feedback exists (Spiral Delay, Fractal's circular delay, inDelay) | https://www.musehub.com/plugin/spiral-delay |
| Saturation of the body of a sound only, the hit kept clean | Transient-split saturation exists (Audio Assault TransFreak, Saturator Pro) | https://producerstack.com/products/saturator-pro-max-for-live |
| Pan a mono line by its own detected pitch, like a piano's spread | Melda's modulators have a pitch mode that can drive pan; frequency panners exist | https://www.meldaproduction.com/tutorials/modulators-pitch-mode |
| Mid and side wound on one core with both windings carrying their sound into one flux | Mid plus side is left and mid less side is right, so it comes to one core on each of left and right, which every stereo transformer emulation does. Made instead as a saturable reactor, one winding steering what the other gets through (Ironside) | plugins/ironside/NOVELTY.md |
