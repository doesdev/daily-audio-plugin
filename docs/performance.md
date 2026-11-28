# What a plugin costs

The rule is the same as for quality: measured, not assumed. This is
what was measured, what was done about it, and what comes next.

## How it is measured

Every plugin's suite runs `is_within_budget`: one instance over a
sung phrase at 44.1, 48 and 96 kHz, in blocks of 64 and 4096, as a
share of real time on one core. To see the numbers:

```sh
cargo test --release -p NAME is_within_budget -- --nocapture
```

On the machine the daily run uses (a 2.3 GHz Xeon, two cores), on
28 November 2026:

| Plugin | 48 kHz, 64 | 96 kHz, 4096 |
| --- | --- | --- |
| Undertow | 0.21% | 0.38% |
| Aftersound | 0.12% | 0.21% |
| Hand-Me-Down | 0.13% | 0.22% |
| Ironside | 0.35% | 0.65% |

A core runs four to five hundred of any of the first three, and
nearly three hundred of Ironside, which runs its core twice (once
heard on the middle, once read by the sides) and works its choke's
corner out again every sample. The ceiling in the
suite is 10%, so a plugin has room to be fifty times dearer than
these, which matters below.

## What was wrong: silence cost more than sound

A sound that dies away never reaches zero. Filters, followers and
strings shrink toward it for ever, and below about 1e-38 a float is
denormal, which a processor handles many times more slowly. Measured
second by second after one note was let go:

| | Playing | Silence after, each second |
| --- | --- | --- |
| Aftersound before | 0.12% | 0.06, 0.47, 1.00, 0.99, 1.00 and on, for good |
| Aftersound now | 0.12% | under the cost of playing |

Eight times the cost of playing, for as long as the instance was
left, on every track that had gone quiet. The runner now tells the
processor to treat such numbers as zero for the length of each block
and puts the thread's own setting back after (`kit::denormal`).
Nothing audible changes: they are some 760 dB down.

The suite gained `a_tail_costs_no_more_than_the_sound`, so a plugin
cannot ship this again, and a test plugin that turns the flushing off
proves the check catches it.

## What was wrong: dropouts at small buffers

Seen in real sessions by the owner, with Hand-Me-Down the worst. The
sound itself was never the cost: at a buffer of 16 its middle block
takes 0.2% of its own length. Two things round the sound were.

**Meters were told to the host every block.** In VST3 a meter reaches
the editor as a parameter change the host queues on the audio thread
and carries to another. The kit sent one whenever a meter had moved,
which for a level is often and for Hand-Me-Down's two, which follow
a note's age and never rest, is every block. At a buffer of 32 that
is 1500 blocks a second, so thousands of changes a second through
the host from one instance. They are now told thirty times a second
at most, counted in samples, whatever the buffer. CLAP was never
affected: its meters do not go through the host.

**A reset cleared megabytes on the audio thread.** A host may reset
a plugin there, as when the transport starts. Hand-Me-Down cleared
its eight seconds of memory each time:

| | Reset | Against one block |
| --- | --- | --- |
| 48 kHz, buffer of 16 | 0.21 ms | 63% |
| 96 kHz, buffer of 32 | 0.62 ms | 186%: a certain dropout |

It now marks its memory as empty without emptying it.

The suite changed to match where the pressure is:

- `is_within_budget` runs at buffers of 16 and 32 as well as 64 and
  4096.
- `a_reset_fits_in_a_small_buffer` gives a reset a quarter of the
  smallest buffer.
- A test holds the host to at most about thirty meter changes a
  second at buffers of 16, 32, 64 and 1024.

Not yet known: whether these two were all of it. They are what
measuring found and they fit what was seen, but nothing has been
confirmed in the session where the dropouts were. If they persist,
the next things to look at are the editor (a web view drawing sixty
times a second beside the audio) and the linking thread.

## What made no difference

Tried on all three plugins, each within the noise of the measurement:

| Build | Undertow | Aftersound | Hand-Me-Down |
| --- | --- | --- | --- |
| As shipped (thin LTO, 4 units) | 0.21% | 0.12% | 0.13% |
| Fat LTO, 1 unit | 0.20% | 0.12% | 0.13% |
| The same, with AVX2 and FMA allowed | 0.21% | 0.12% | 0.17% |

So neither was changed. The reason is the shape of the work, not the
compiler: a filter or a valve with memory needs sample n before it
can start sample n + 1, and there is nothing for a wider instruction
to do side by side. Vectorising code like that is effort for no
gain, and at 0.2% there is little left to gain.

## What oversampling costs

Built on 2026-11-28 (`docs/quality.md`). One valve, mono, at 48 kHz,
as a share of one core:

| Rate | Filters alone | With the valve |
| --- | --- | --- |
| 1 | 0.01% | 0.20% |
| 2 | 0.11% | 0.45% |
| 4 | 0.25% | 0.92% |
| 8 | 0.49% | 1.85% |
| 16 | 1.00% | 3.72% |

The steeper filters for rendering cost within a tenth of these. The
valve is three quarters of the cost at every rate: it is a `tanh`
for every sample, and a faster stand-in for `tanh` is a trade of
quality for speed that is not made here. Auto is what keeps the cost
down: a bass, or anything gently driven, runs at the plain rate.

Two things were measured on the way:

- **A second build for AVX2 was no faster.** The filter sums were
  laid out eight abreast, compiled once for any x86-64 and once for
  AVX2, and chosen by asking the processor. A sum of 56 took 8.6 ns
  the plain way and 9.2 ns the wide way, and the filters as a whole
  were the same to within the noise. The plain build already does
  them four at a time, and at these lengths the adding up at the end
  is what takes the time. The second build was taken out.
- **Doing the sum before the write halved the filters.** Each sum
  reads eight samples at once, the last of them the one just written,
  and a processor cannot read across a sample it has not finished
  writing: it waits. Summing over the sample about to be replaced and
  putting the difference right afterwards took the filters at 16
  times from 1.8% to 1.0%.

## Where vectors will pay, and what comes next

Vector instructions do many of the same thing at once. Two kinds of
work in an audio plugin have that shape, and the kit has neither yet:

1. **Oversampling filters.** Built, and measured above: the plain
   build's four at a time is all these short sums can use.
2. **Voices.** An instrument's notes are the same sum done once a
   note. Aftersound's strings could run four or eight notes abreast.

How it will be written, following the survey the owner pointed to
(Shnatsel, "The state of SIMD in Rust in 2025"):

- Stable Rust will not vectorise a float sum by itself, since doing
  so changes the order of the additions. So the sums are written as
  eight running totals side by side, which it will.
- One build must run on every machine. Each hot routine is compiled
  twice, for the baseline and for AVX2 with FMA, and the faster is
  chosen once at start by asking the processor. No dependency is
  needed for that; a portable SIMD crate with dispatch built in
  (`pulp`, `fearless_simd`) is the step after, if hand-laid lanes
  prove too clumsy.
- Each such routine has a test that both versions give the same
  numbers, and a measurement beside it here.

What is not on the list: faster approximations of `tanh` and the
like. They are where most plugins buy speed, and they are a trade of
quality for it, which is not wanted here.

## Open

- These numbers are from one Linux machine. Nothing has been timed on
  Windows or in a host, and a host's own cost for each parameter
  change is not something a test here can see.
- Oversampling's own open ends are in `docs/quality.md`.
