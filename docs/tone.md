# The tube and the transformer

Two windows are the same in every plugin: Tube and Transformer. This
says what each lever in them stands for, why it is a lever, and how
far the evidence goes. It is the reasoning behind `kit/dsp/src/tube.rs`
and `kit/dsp/src/transformer.rs`; change it when they change.

The aim is not to copy part numbers. It is to find the few causes
that make one valve stage or one transformer sound unlike another,
give each cause a lever, and offer a named setting of the levers (a
character) for the kinds of part people know. A character is a place
to start refining from.

## Tube

Four causes change what a valve stage adds: how it is wired, where it
rests, what its grid does when driven, and how much it feeds back on
itself. The supply and the top end shape the rest.

| Lever | Stands for | What is heard | At rest |
| --- | --- | --- | --- |
| Stage | Triode, a pentode strapped as one, or a pentode | Even harmonics from the first, or odd ones and a later knee | Pentode |
| Bias | Where on its curve the valve rests | How much second harmonic, which half clips first, starved or full | 0% |
| Grid | The grid starting to conduct | The top half flattening harder than the bottom | 0% |
| Bloom | Grid current charging the capacitor that feeds it | A loud sound pushes the rest down, and it comes back over 30 to 300 ms | 0% |
| Feedback | A cathode resistor left unbypassed | Cleaner and firmer up to the ceiling, then a harder knee | 0% |
| Sag | The supply drooping under load | A held loud sound eases back | 0% |
| Glow | More top into the curve, the same off after | What is added sits higher | 0% |

**Stage** is the largest switch. A triode's current follows the
three-halves power of how far it is above cutoff, so its curve is
lopsided from the first and what it adds is mostly the second
harmonic. A pentode's screen holds its current nearly independent of
the anode, so its curve is even either side and what it adds is mostly
the third ([r-type](http://www.r-type.org/articles/art-123e.htm),
[Blencowe](https://www.valvewizard.co.uk/Common_Gain_Stage.pdf)). An
EF86 is both: a pentode of high gain, or strapped as a triode with a
mu of 38, which is how the U67 runs it
([Neumann](https://manualmachine.com/neumann/u67/812626-user-manual/)).
A triode's top is also a little darker, from the Miller effect, which
the screen spares a pentode: the model gives a triode one pole at
14 kHz, strapped one at 20 kHz, a pentode none.

A pentode at rest is a plain `tanh`, which is why it is the rest: a
plugin that gains the window sounds as it did.

**Bias** only ever adds to what the plugin itself does with the
resting point. Undertow rides it with the bass, and a test holds that
the window's Bias moves where that ride is centred and takes nothing
from it. Hardware that exposes bias shows how much it matters: the
Culture Vulture's manual puts its cleanest setting at 0.25 mA, with
less current thin and quick to break up and more of it fat
([Thermionic Culture](https://www.thermionicculture.com/images/stories/virtuemart/product/CultureVultureValveDistortionUnitManual9.pdf)).

**Bloom** is the memory the first version lacked. The times are
design values, not measured from any one circuit.

**Feedback** is solved, not delayed by a sample: a delay would turn
it into a treble boost. The bypass capacitor's low shelf is not
modelled.

Not built: the screen sagging with the supply in a pentode, and
noise, hum and microphony. Microphony is the likeliest physical cause
of one maker's valve sounding more open than another's.

### Characters

| Character | After | How well founded |
| --- | --- | --- |
| EF86 pentode, open | An EF86 as a pentode in a microphone or line amplifier, with some sheen | The wiring is; "open" is a reputation |
| EF86 pentode, tight | The same with more feedback and less bloom | The same |
| EF86 strapped | An EF86 as a triode with feedback, as in a U67 | Well: the circuit is documented |
| 12AX7 stage | A high gain triode stage with its cathode bypassed | Well |
| Starved plate | A triode run on too little current | Well: thin, gated, sputtering |
| Push-pull pair | Two valves in push-pull, where the second harmonic cancels | Well |

On makers: Telefunken EF86s are said to have a sheen and Valvos to be
tight. Construction does differ (a solid plate against a short mesh,
and how prone each is to ring), but no published measurement of one
maker against another was found, and listeners' reports contradict
each other. So the two EF86 characters are settings of causes that
could plausibly differ (rest, feedback, bloom, where the harmonics
sit), not models of two factories.

## Transformer

Everything follows from one fact: a core saturates on flux, and flux
is voltage added up over time. A low note fills the core sooner than
a high one at the same level, so what a transformer adds rises as the
frequency falls. The model adds the sound up into flux, bends the
flux, and turns only the bend back into sound, so what the core
leaves alone comes out exactly as it went in.

| Lever | Stands for | What is heard | At rest |
| --- | --- | --- | --- |
| Drive | Level into the core | The lows thicken and then fold | 0% |
| Size | How much iron: the note at which a full level fills it | Where the thickening starts. Lower is bigger | 60 Hz |
| Iron | Nickel to steel | Clean then a sudden knee, or an earlier, softer bend and a grain on quiet sounds | Nickel |
| Source | The resistance of what feeds it | More of the bend, and less bass until the sound is loud | 0% |
| Grip | A standing current in a gapped core | Even harmonics, and some bass given up | 0% |
| Top | The windings' resonance brought down into hearing | Where the top turns over | 0% |
| Ring | How little the load damps it | A smooth roll, or a peak with an edge | 20% |

**Source** is the lever most transformer plugins lack and the one the
best evidence is for. The lows are lost where the core's reactance
falls to the source's resistance, and the distortion heard is the
core's magnetising current times that resistance, so the same iron is
clean from a low source and coloured from a high one
([Whitlock, for Jensen](https://www.jensen-transformers.com/wp-content/uploads/2014/09/Audio-Transformers-Chapter.pdf)).
A core's inductance also rises with level before it saturates, which
is why a loud low note gets through a high source that holds a quiet
one back.

**Iron**: steel distorts low notes even at low level, and does not
get much worse until it saturates, which is at a higher level; a
high-nickel core is clean until its knee
([Whitlock](https://www.audiotechnology.com/features/more-than-meets-the-eye)).
Steel's lag is a simple play operator here, not the Jiles-Atherton
model, which is the better one and costs a solver each sample
([Holters and Zölzer](https://www.hsu-hh.de/ant/wp-content/uploads/sites/699/2017/10/Holters_jamodel_DAFx16.pdf)).

**Grip**: direct current in a winding reduces inductance and adds
even harmonics (Whitlock). Both are coupled inside the one lever.

**Top and Ring**: in hardware this resonance, from leakage inductance
and winding capacitance, sits from about 30 to over 100 kHz, and the
load decides whether it peaks or rolls. Top brings it down as far as
8 kHz, which is further than any real part: it is a lever for the
effect, not a measurement.

Not built: oversampling. The bend acts mostly on the lows, so what it
folds back is small, but it has not been measured here.

### Characters

| Character | After | What is known | How well founded |
| --- | --- | --- | --- |
| Mic output, T14 kind | The Haufe T14/1, the output transformer of the C12 and ELA M 251 | 11.5 to 1, a primary near 2.95 kohm, fed from a valve's plate | Weakly: no core alloy, inductance or level limit was found. Its settings follow from the high source and the high ratio |
| Ouncer, O-12 kind | The UTC O-12, the 1176's input through revision F | One ounce, rated to +8 dBm, 30 Hz to 20 kHz | Fairly: the catalog figures say it fills at ordinary line level |
| Gapped output, LO1166 kind | The Marinair LO1166 after a Neve class A output stage | Gapped, about 80 mA standing in it, a low source, damped by a network | Fairly: steel is from a replica maker, the rest from measurements on forums |
| Clean nickel input | A high-nickel input transformer of the Jensen kind | 0.002% at 20 Hz and +4 dBu | Well |
| Big steel output | A large steel output transformer | None verified | Weakly |

One correction to what was asked for: the T14/1 turned up as a
microphone's output transformer, not a console's. If the one meant is
in other gear, the character should be looked at again.

Sources for the three parts:
[T14/1 resistances](https://groupdiy.com/threads/haufe-t14-1-dc-resistance-comparision.60951/),
[UTC catalog, 1963](http://bitsavers.trailing-edge.com/components/utc/1963_UTC_Catalog.pdf),
[1176 revisions](https://www.mixonline.com/recording/1176-revision-history-372048),
[LO1166 gap and current](https://groupdiy.com/threads/lo-1166-gapped-neve-output-iron.6628/).
The nickel figure is from Jensen's JT-11-BMCF data sheet.

## How to check a change

- A low sine at rising level, at 20, 40 and 100 Hz: the third
  harmonic should come in an octave higher for each doubling of
  level.
- A quiet low sine through steel and through nickel.
- A square wave through Top with Ring up and down.
- A loud burst and then a quiet tone through Bloom: the quiet tone's
  second harmonic should fade over the release.
- Nothing here has been compared with the hardware by ear or by
  measurement. Whoever owns a T14/1, an O-12 and an LO1166 is the
  test.
