# What is new about Hand-Me-Down

## What is new

Hand-Me-Down hears where each note starts, keeps that note's own hit,
and then plays the body that an earlier note left behind, from the
same point in its life. A sound's attack and its sustain stop
belonging to the same note: on a melodic line the pluck is now and
the ring is one note late, so every note is struck at its own pitch
and sings at the last one's. Its levers are the ones the listener
reaches for: hit and body split apart, and the handed-down body put
into the middle, the sides or both, so a part can hold its own body
in the centre and its predecessor's out wide. Back reaches up to four
notes back.

Transient and sustain splitters are well trodden. What none found
does is recombine them across notes: take the attack of one event and
the sustain of another event of the same track, in time, as it plays.

## Nearest existing things

- [Image-Line Transmitter](https://www.image-line.com/fl-studio-learning/fl-studio-online-manual/html/plugins/Transmitter.htm):
  splits a sound into transient and sustain with complementary
  envelopes and processes each apart, with a sidechain for the
  detector. Both halves always come from the same moment of the same
  input; it never carries one note's sustain onto another note.
- [NoiseWorks TranSplit](https://www.kvraudio.com/product/transplit-by-noiseworks)
  and [Matthieu Brucher's ATK Transient Splitter](https://www.kvraudio.com/product/atktransientsplitter-by-matthieu-brucher):
  split transient from sustain for separate treatment. Same moment,
  same note.
- [iZotope Neutron's Transient Shaper](https://s3.amazonaws.com/izotopedownloads/docs/neutron202/en/transient-shaper/index.html):
  turns attack and sustain up or down by band. Gain only.
- [Low End Candy Slingshot](https://www.mixonline.com/technology/news-products/low-end-candy-fires-off-slingshot-plug-in):
  a reverse delay that lands its echoes on the next transient. It
  moves a sound towards a hit; it does not swap the bodies of notes.
- Sample synthesisers that join a recorded attack to a looped sustain,
  and articulation joining in patents such as
  [US 7,432,435 (Yamaha, tone synthesis)](https://image-ppubs.uspto.gov/dirsearch-public/print/downloadPdf/7432435):
  these choose stored samples for notes they are told to play. They do
  not process a live track, and they join a note's attack to that
  same note's sustain sample.
- Slicers and beat repeaters (Sugar Bytes Looperator, FL Slicex,
  Ableton Beat Repeat): cut audio on a grid or at onsets and replay
  whole slices. None keeps the new hit and replaces only what follows
  it.
- Onset delays such as RhythmEcho (already in Dropped for another
  idea) time an echo by the detected beat; an echo is added on top,
  where here the note's own body is taken away and replaced.

## Where I looked

On 2026-11-27: web searches for transient and sustain swapping between
notes, an attack of one note with the body of another, onset slicing
that replaces a slice's tail with the previous slice, delays of the
sustain alone behind the transient, "one note behind" and "previous
note" echoes, and DAFx work on attack and sustain recombination
(found only separation papers, such as real-time transient separation
by multiresolution analysis). Read in full: the Transmitter manual
page, the TranSplit KVR pages, the Neutron Transient Shaper manual
page, and both Slingshot announcements. Google Patents style searches
for a transient portion of a current note combined with the sustain
of a preceding note found only sample synthesis patents, which select
stored waves and work on notes they are told to play. GitHub
repository search for transient and sustain swaps found nothing.

Not covered: Max for Live devices and Reaktor ensembles one by one,
modular hardware, and anything not described in words online.

## What would change this

A plugin, device or paper that, on a live signal, detects note starts
and plays a later note's attack with an earlier note's sustain. A
granular or slicing device with a mode that keeps each new slice's
first few milliseconds and fills the rest from an older slice would
be the same idea under another name, and the idea would be dropped.

## Against the best of its kind

Its kind is the transient and sustain splitter; Transmitter and
Neutron's Transient Shaper are the leaders (`benchmark.json`).

- **Picture**: the family line, notes along time with the hit of each
  and an arc from each body back to the note it came from. Nothing in
  the field draws where a sustain came from, because nothing else
  moves one. Transmitter's scrolling display draws its two envelopes
  over the live waveform; this one draws the notes from the meters'
  last four seconds and does not show the waveform itself, which is
  under. With nothing playing it draws a line of plucks from the
  settings rather than anything heard.
- **Precision, compare, explains, fits, presets**: the kit's, which
  meet the bar; A and B and the undo history are forgotten when the
  editor closes, as for every plugin on the kit.
- **Character**: the house look.
- Under: on a dense picture of many short notes the meters' readings
  come a few times a second, so notes closer than a reading are drawn
  as one. No per band split (Neutron splits by band), and no lookahead,
  so the first fraction of a millisecond of a hit can be missed on a
  slow attack; it was left out to keep the plugin at zero latency, as
  the kit's bypass does not delay the dry sound.
