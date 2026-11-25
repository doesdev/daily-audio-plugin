# What is new about Undertow

## What is new

Undertow puts the sides of a stereo sound through a valve-like curve
whose resting point is set, sample by sample, by the low end of the
middle. The middle passes untouched. When the bass swings up, the
sides are pushed up the curve; when it swings down, they are pushed
down it. Off its centre the curve is lopsided, so the sides pick up
even harmonics and lose a little level in time with the bass wave
itself, not with an envelope of it. What the bass put there is taken
away again after the curve, so nothing of the middle leaks into the
sides, and a mono sound comes out exactly as it went in. The one idea:
**the middle's own waveform biases the sides' valve.** Steer turns it
round, so the sides bias the middle instead.

This is built on two of the levers in `ideas/backlog.md`: mid and
side, with one steering what is done to the other, and the bloom and
sheen of valves in the upper harmonics. It uses the mid and side lever
in a way no product found does: as the operating point of a
nonlinearity, at audio rate, rather than as a level that sets a gain.

## Nearest existing things

- [Panoramic, Panorama Audio Labs](https://panoramalabs.com.au/panoramic/):
  its Steal section "derive[s] harmonics from the Mid channel and
  introduce[s] them into the stereo image". The middle there is the
  material that is distorted. In Undertow the middle is never
  distorted and never heard in the sides: it only decides where on the
  curve the sides sit, so what is added is the sides' own harmonics.
- [MSaturatorMB, MeldaProduction](https://www.meldaproduction.com/MSaturatorMB):
  saturation in mid and side, with modulators that can follow a level.
  A follower of the middle could ride the drive of the sides, but that
  is an envelope moving a gain, slower than the bass and the same for
  both halves of its cycle. Undertow follows the wave, so the two
  halves of each bass cycle push the sides opposite ways.
- [Kilohearts Distortion](https://kilohearts.com/docs/snapins): a
  static Bias, and a Spread that gives left and right different bias.
  Neither takes its bias from a signal.
- [Phase Plant waveshaping, Future Music](https://www.pressreader.com/australia/future-music-9629/20200501/283901640117411):
  bias "adds a DC offset before the distortion" and can be modulated
  for a pulse width effect. The modulation is of one signal's own
  shaper, not one signal setting another's.
- [Native Instruments Dirt](https://native-instruments.com/ni-tech-manuals/crush-pack-manual/en/dirt)
  and [Ohmicide](https://ohmicide-eurorack-manual.readthedocs.io/en/latest/reference/index.html):
  bias as a fixed asymmetry, or Ohmicide's envelope driven modes. No
  second signal sets it.
- [US 9,883,318, Bongiovi Acoustics](https://patents.google.com/patent/US9883318):
  active, expected to run to 2033. Its claims split the input into a
  low and a higher band, split the higher band into mid and side, make
  a detection signal from the mid and use it to set the gain of the
  side through a compression module, with make-up. Undertow does not
  split the input into bands, has no detector or compressor, and does
  not make the side up to its own level: it is a waveshaper whose
  operating point is the mid's own low wave. A run is not a lawyer;
  this is the one patent found near it, and why it was judged clear.
- Modular synthesis: any audio signal can be patched into a
  distortion's bias input. Nothing found packages it as a stereo tool
  that keeps mono and the middle untouched, but a person with a
  modular has been able to do something like it.

## Where I looked

On 2026-11-25, by what it does rather than what it is called:

- Web and shops: "saturation plugin bias modulated by audio signal mid
  channel biases side channel"; "distortion plugin bias modulation by
  envelope or audio rate sidechain"; "saturation plugin harmonics
  generated from mid placed in side channel"; "harmonic stereo
  widener adds harmonics to the side signal"; KVR, Plugin Boutique and
  maker pages that turned up (Panoramic, Airy, G-Sonique, Wider,
  MSaturatorMB, Saturn, Kilohearts, NI, Ohm Force, Renegade Sound
  Labs Dual Core Saturator, SKnote AnyTesla).
- Airwindows: a search for saturation where the two channels act on
  each other found nothing of the kind.
- Papers: DAFx and AES on waveshaper bias set by a second signal, and
  on stereo saturation; nothing found sets one signal's operating
  point from another.
- Patents: by technique ("nonlinear processing side signal mid signal
  offset added before nonlinearity"), which found the Bongiovi family
  above.

Ideas dropped on the way today, each written in the backlog: a delay
timed by the gap between hits (RhythmEcho), a mid and side envelope
trade (envelope imposers), harmonics of the mid put in the side
(Panoramic), a delay whose repeats turn through the stereo field
(Spiral Delay and others), saturation of the body only
(TransFreak) and pan by pitch (Melda's pitch modulators). A shared
transformer core for mid and side was set aside while US 12,425,770
could not be read; read since, its claims need physical hardware, and
the idea is back in the backlog.

## What would change this

A plugin or a paper that sets the bias of one channel's shaper from
another channel's audio at audio rate, in mid and side or left and
right. A reading of US 9,883,318 by someone who knows patents that
says a waveshaper's slope counts as its "gain".

## Against the best of its kind

The kind is stereo saturation: [FabFilter Saturn 2](https://www.fabfilter.com/products/saturn-2-multiband-distortion-saturation-plug-in)
and [Panoramic](https://panoramalabs.com.au/panoramic/)
(`benchmark.json` has each of the seven points).

- **The picture**: the curve itself, with the point the bass has put
  the sides at riding along it and the swing of the sides drawn
  round it. Saturn shows bands and a spectrum; Panoramic shows a
  stereo field. Neither shows where on its curve a sound is.
- **Precision**, **compare and recover**, **explains itself**,
  **fits the screen**: the kit's, which meet the leaders.
- **Character**: the house look, with the riding point purple while
  Push or Follow is held.
- **Presets**: four of its own.
- **Where it is under**: no modulation, which Saturn has; one band,
  where Saturn has six; no bands of its own at all; A and B and the
  undo history are forgotten when the editor closes. It does not show
  the sides' own signal live: the kit sends the editor one channel,
  which here is the middle.
