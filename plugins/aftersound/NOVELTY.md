# What is new about Aftersound

## What is new

Aftersound is an instrument whose every note is two strings, one
string in each channel, joined at a shared bridge. Struck together,
the two strings move as one, and the bridge drinks that motion fast:
that is the piano's prompt sound, and because the two channels are
the same it sits in the middle. A hair of mistuning lets the strings
drift apart; motion where they swing against each other barely moves
the bridge, so it rings on for a long time: that is the aftersound,
and because the channels are opposite it is the side. The one idea:
**the two decays of coupled strings, heard as space.** In mid and side
terms the physics is exact: the middle is the in-phase mode, the side
is the out-of-phase mode, so a note starts narrow and blooms wide as it
dies away, by the same law that gives a piano its double decay.

This is built on two of the levers in `ideas/backlog.md`: instruments,
and mid and side used creatively, here as the thing the instrument
makes rather than a treatment applied after it. The listener asked for
a voice whose middle and sides are made apart; here they are the two
modes of one pair of strings, and the settings that shape each are the
physical ones (how hard the bridge pulls, how far apart the strings
are tuned, which string the hammer hits harder).

## Nearest existing things

- [Coupled piano strings, Julius O. Smith, Physical Audio Signal Processing](https://ccrma.stanford.edu/~jos/pasp05/Coupled_Piano_Strings.html)
  and [Weinreich's account of the double decay, KTH](https://www.speech.kth.se/music/5_lectures/weinreic/weinreic.html):
  the theory Aftersound rests on, coupled waveguides at a common
  bridge giving a fast and a slow decay. There the output is the
  bridge and the soundboard: the strings are summed, so the aftersound
  is heard only as a weak tail in one channel. Aftersound listens to
  the two strings themselves, one per channel, which turns the two
  decays into two places.
- [Two planes of vibration, Lee and Smith, REALSIMPLE](https://ccrma.stanford.edu/realsimple/phys_mod_overview/Modeling_Two_Planes_Vibration.html)
  and the [coupled strings in Faust](https://ccrma.stanford.edu/realsimple/faust_strings/Coupled_Strings_Faust.html):
  two coupled waveguides with two loop filters for "Two-Stage Decay
  and beating effects". The two are planes of one string and are not
  routed to stereo; nothing there maps the modes to mid and side.
- [Resonarium, Gabriel Soule](https://web.stanford.edu/~gns/resonarium.pdf):
  a VST3 with resonator banks "interlinked" through "a shared bridge
  filter". Its stereo is the whole network duplicated, "each channel
  has its own independent state"; the coupling is inside a channel,
  never across the two.
- [Pianoteq, Modartt](https://www.modartt.com/pianoteq_features):
  physically modelled piano with "unison width" and "direct sound
  duration" among its settings, and a [forum thread on unison width](https://forum.modartt.com/viewtopic.php?id=1650)
  that treats the strings of a note as detuned unisons. Its stereo is
  from modelled microphones over the whole instrument; no string has a
  channel of its own and the two decays are not placed apart.
- [Chromaphone 3, AAS](https://www.pluginboutique.com/product/1-Instruments/4-Synth/6999-Chromaphone-3):
  two layers of resonators with coupling between them. Its strings
  are a resonator type; nothing found sends coupled modes to the
  middle and the sides.
- [Plucked!, Image-Line](https://www.image-line.com/fl-studio-learning/fl-studio-online-manual/html/plugins/Plucked!.htm):
  a Karplus-Strong string with a Widen control that "enriches the
  stereo panorama". One string, widened after the fact.
- [US 5,212,334, Stanford](https://patents.google.com/patent/US5212334):
  digital waveguide networks, the technique under every string here.
  Filed in the late 1980s and long expired.

## Where I looked

On 2026-11-26, by what it does rather than what it is called:

- Web and shops: "plugin coupled strings one string per stereo channel
  symmetric mode mid antisymmetric side aftersound"; "Pianoteq unison
  strings stereo output each string left right channel coupling";
  "synthesizer plugin coupled strings physical model double decay
  stereo width plucked string"; "physical modelling string synth left
  and right strings coupled bridge stereo image two strings"; KVR and
  maker pages that turned up (Pianoteq, Chromaphone, Strum, Plucked!,
  PartialString, Physical Audio Preparation 2 and Modus, Rob Papen
  WirePluck, kenotron strings-vst).
- Papers: CCRMA (Smith, Lee), KTH on Weinreich, a DAFx and coupled
  strings search ("coupled strings stereo rendering symmetric
  antisymmetric modes spatial two-stage decay"), which found modal and
  waveguide models, all of them summed to the bridge.
- Patents: by technique ("string synthesis two coupled waveguides
  output left channel right channel stereo"), which found the
  Stanford waveguide family (expired) and old Yamaha tone generators.

Two things turned up that could not be read: a pull request called
"Coupled strings: sympathetic resonance and parallel branches" in
another project (`agentchieflou/noprod`, the fetch was rate limited)
and Sympiano, a grand piano's insides as a reverb (the page would not
load). Both looked, from their titles, to be resonance effects rather
than strings placed per channel; a run that can read them should.

## What would change this

An instrument, a patch or a paper that sends each of a pair of coupled
strings to its own channel, or that renders the in-phase and
out-of-phase modes of coupled strings as the middle and the sides. A
modular patch of two coupled delay loops panned hard apart would be
close in sound, though nothing found presents it.

## Against the best of its kind

The kind is the physically modelled string instrument: Pianoteq and
Chromaphone 3 (`benchmark.json` has each of the seven points).

- **The picture**: the two decays themselves, worked out from the
  settings by the same law the sound uses: the middle falling fast,
  the side rising out of nothing and lingering, the moment the side
  takes over marked, and each note's live levels laid over them.
  Pianoteq shows its panels and a blueprint; Chromaphone its
  resonators. Neither shows a note's decay, or where it is in space.
- **Precision**, **compare and recover**, **explains itself**,
  **fits the screen**: the kit's, which meet the leaders. Pianoteq has
  each of these as well.
- **Character**: the house look; the curve a setting shapes turns
  purple while it is held.
- **Presets**: four of its own. Pianoteq and Chromaphone ship large
  libraries.
- **Where it is under**: one kind of string and hammer, where
  Pianoteq models whole instruments and Chromaphone nine resonators;
  the sustain pedal reaches only the CLAP, since the kit does not yet
  hand a VST3 its MIDI controllers; no modulation; no soundboard or
  body; at its fundamental the picture is exact, but the upper
  partials spread sooner than it shows; A and B and the undo history
  are forgotten when the editor closes.
