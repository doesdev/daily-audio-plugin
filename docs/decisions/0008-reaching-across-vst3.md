# 0008: How the editor reaches the audio side in VST3

2026-11-25

VST3 keeps a plugin in two halves, a processor and a controller, and
the controller hears of the processor only through parameters. Two
things the kit's benchmarks called for do not fit through that: the
sound itself, for a picture to draw, and MIDI controllers, for a
setting to learn. CLAP has neither problem: its editor and its audio
thread share memory, and controllers arrive as events.

**The sound.** When the host connects the two halves, the processor
sends the controller one message carrying a counted reference to its
`Scope` (`kit/vst3/src/connect.rs`). From then on the editor reads it
directly, with no lock. This assumes both halves are in one process,
which is how hosts load them; the message names its process and one
from elsewhere is refused, leaving the editor without a waveform and
nothing worse. The alternative, a stream of messages from a thread of
the processor's own, was more to get wrong for the same picture.

**Controllers.** VST3 hands a plugin no controller messages. The host
asks the controller, through `IMidiMapping`, which parameter each
controller should move, and then moves it. So the kit adds 128 hidden
parameters, one standing in for each controller, maps every
controller to its own, and the processor reads a change to one as the
controller it is. Their ids start at `CC_PARAMS`, well away from any
setting's. They are hidden and cannot be automated, so a host's
parameter list is not filled with them, but a host that ignores the
hidden flag will show them.

**What it costs.** Every plugin now takes MIDI in, in both formats,
whether or not it plays notes: it is how a controller reaches it.
Notes still go only to a plugin that asks for them. Learning is one
controller to a setting, on any channel, 7 bits wide.

**What is saved.** A setting that has learned a controller gets a
second pair in the saved state, its id with `LEARNT` set. A build
from before this reads past it.
