# What is new about Ironside

## What is new

Ironside winds the sides on a core that the middle fills. The middle
passes through a transformer's core, and the sides pass through a
second winding on the same core, a choke across their path. While
the core is empty that choke has a lot of inductance, and the sides
lose only what is below Corner. When the middle's bass fills the
core the inductance falls away, so the sides' corner climbs. It does
this on each swing of the bass, with no envelope in between. That is
a saturable reactor, the part that once let a small current steer a
large one in stage lighting dimmers and magnetic amplifiers. Here it
is used so that one half of a stereo sound sets how much of the
other's low end gets through.

Three things follow from its being a core and not a detector. A core
fills on flux, which is the sound added up over time, so a low note
in the middle thins the sides far more than a high note at the same
level does. The thinning happens twice in every wave of the bass, at
its crests, so at Recover nought the sides are opened and shut 120
times a second under a 60 Hz bass. That is heard as a grain in the
stereo image, not as a duck. And a steel core lags and closes on
quieter sounds than a nickel one.

The backlog's idea was "mid and side wound on one shared core, so
the middle's bass saturates the core the sides pass through". Taken
literally, with both windings carrying their sound into one flux,
that comes to the same thing as one core on the left and another on
the right, since mid plus side is left and mid less side is right.
Every stereo transformer emulation already does that. The reactor is
the version that is new: one winding sets the state of the core and
the other reads it, and nothing of the first is heard in the second.

## Nearest existing things

- **Undertow** (this repo, 2026-11-25). The middle's low end biases
  the sides' valve, sample by sample, which gives the sides even
  harmonics in time with the bass. Ironside adds no harmonics to the
  sides. It moves their low corner. Its steering comes from flux, so
  the depth of the note counts, which in Undertow it does not: there
  a low-pass sets what pushes and every note under it pushes alike.
- **US 9,883,318 B2, Bongiovi Acoustics** (priority 2013, shown as
  active by Google until 2033). It splits the input at a crossover,
  turns the part above it into middle and side, runs the middle
  through a detector, sets the gain of the side from that, and then
  raises the side to at least where it was. Ironside has no
  crossover, gives no gain to the side and raises nothing. It moves
  a filter's corner on the side, at audio rate, from a model of a
  magnetic core, and the corner is held under Ceiling so the sides'
  highs are never ducked. This was read claim by claim and does not
  seem to cover Ironside. A run is not a lawyer and this is not legal
  advice. https://patents.google.com/patent/US9883318
- **Moro and McPherson, DAFx-15**, "Approximating non-linear
  inductors using time-variant linear filters". The same way of
  working: a filter whose inductance is worked out again every sample
  from a saturating curve. But there the current that saturates the
  inductor is the one passing through it, in a high-pass or a wah.
  In Ironside the sound that saturates the core and the sound that
  is filtered are different sounds.
  https://dafx.de/paper-archive/details/SGj2HLfMOvHlajZxdjVo8w
- **Najnudel, Muller, Helie and Roze, DAFx-20**, "A Power-Balanced
  Dynamic Model of Ferromagnetic Coils". A coil with hysteresis, fed
  by its own current. It names a transformer of two coupled coils as
  future work, but not one winding steering another.
  https://dafx.de/paper-archive/details/M48r2d8e7djf8l-SffOBVw
- **RJ Studios SideMinder ME2**. It sets stereo width dynamically,
  like a limiter, in up to three bands. A stereo sidechain steers it
  "in complementary response" to another track's width. It acts on
  width as a level with a release, not at the rate of the bass, and
  not from the middle's flux.
  https://www.kvraudio.com/news/rj-studios-releases-sideminder-me2-with-fit-it-in-the-mix-widthwise-sidechain-input-52452
- **Goodhertz Midside**. Its Mono Below takes the sides' lows away
  below a fixed frequency, with Bass Makeup put back in the middle.
  It is static. Ironside's corner is Corner at rest and climbs only
  when the middle fills the core. https://manuals.goodhertz.com/midside
- **Transformer saturators** (Plugin Alliance SPL Iron, Bram's Iron
  Transformer, Black Rooster Magnetite and others). Each one bends
  the sound in its own core, per channel. None of the pages found
  says that one channel's flux sets what another channel gets
  through. https://www.kvraudio.com/product/iron-transformer-by-bram

## Where I looked

- The web, on 2026-11-28: "saturable reactor" with plugin, audio and
  VST; "magnetic amplifier" as an audio compressor or gain control;
  dynamic stereo width with the side ducked by the bass or keyed from
  the middle; a high-pass on the sides keyed from the middle; plugins
  named Ironside.
- KVR and Plugin Boutique, through those searches: transformer
  saturators, mid and side tools, width tools.
- GitHub, through a search for transformer and hysteresis plugins
  with mid and side or cross modulation. Found ChowTape (the
  hysteresis of tape) and Flux-Audio's hysteresis, both with one
  signal through its own core.
- Papers: DAFx, through searches on saturable and non-linear
  inductors and ferromagnetic coils. Two papers read (above). arXiv
  through the same searches, with nothing nearer.
- Patents, by technique: a filter or gain on the side controlled by
  the middle, saturable inductance. US 9,883,318 was read.
  US 8,804,985 ("high pass filtering with smart saturation") came up
  and could not be opened, so its claims are not checked. Its title
  is about saturating a high-pass's own state, not one signal
  steering another. The saturable reactor itself dates from the
  early 1900s and the magnetic amplifier patents found are from the
  1950s, long expired.
- The backlog's own search for the shared core on 2026-11-25, which
  found AnyTesla and RSL Dual Core Saturator keeping one core per
  channel or per band, and read US 12,425,770 B1 (a physical
  magnetic device; a model in software is outside its claims).

## What would change this

- A plugin, hardware unit or paper in which one signal saturates a
  core, or any non-linear element, and that sets a filter on a
  different signal, sample by sample. In particular a mid and side
  tool that does it.
- A dynamic EQ that can key a band on the side from the middle with
  attack and release near nought. That would get close to Recover at
  nought, but it would not have the flux: a detector does not care
  how deep the note is.
- A reading of US 8,804,985, or a later Bongiovi patent, that
  covers a filter on the side as well as a gain.

## Against the best of its kind

Its kind is stereo processing with a transformer's colour. The
leaders in `benchmark.json` are FabFilter Saturn 2, Goodhertz
Midside and SideMinder ME2.

- **The picture.** Above the bar. The sides' own low corner is drawn
  as a response curve, dashed where it sits with the core empty and
  solid where the middle has pushed it now. The span the bass took
  is struck through in columns. Beside it the core's own curve shows
  how far up the middle has filled it. Saturn draws bands over a
  spectrum and Midside a goniometer. Neither can show one half
  steering the other, because neither does.
- **Precision.** At the bar: the kit's values.
- **Compare and recover.** At the bar with Saturn (undo, redo, A/B).
  Midside has A/B and no undo. A/B and undo are forgotten when the
  editor is closed, which is the kit's gap.
- **Explains itself.** At the bar: the foot of the sheet.
- **Fits the screen.** At the bar: scales to any window, three
  densities.
- **Character.** The house look.
- **Presets.** Under. Four presets against Midside's twenty-six and
  Saturn's library. Enough to show what it is for, not a library.
- **What the picture cannot show yet.** The opening and shutting at
  each swing of the bass is faster than the meters the page is sent
  (thirty times a second), so the picture shows the corner where it
  is held. The swing itself is heard, not seen. Next: work the core
  out from the sound the page is sent, held to the plugin by a test
  on each side.
