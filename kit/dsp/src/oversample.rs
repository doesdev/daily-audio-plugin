//! Running a bend at several times the sample rate, so the harmonics
//! it makes above half the rate are filtered off before they can fold
//! back down as tones that were never played (`docs/quality.md`).
//!
//! `Oversampled<T>` holds the thing that bends (a valve, a core) and
//! runs it at 1, 2, 4, 8 or 16 times the rate between two filters.
//! Three things about it are promises, each with a test:
//!
//! - **The delay is one whole number of samples whatever the rate.**
//!   A plugin tells its host that number once, and moving between
//!   rates never moves the sound in time.
//! - **Changing rate is not heard.** The new rate is run beside the
//!   old until its filters are full, and then the two are crossed.
//! - **A second signal arrives with the sound.** A bend steered by
//!   something else (Undertow's bias) gets it at the high rate and
//!   as late as the sound itself is by then.
//!
//! Each doubling is one half-band filter: every other tap of it is
//! nought, so going up costs one sum a pair of samples and coming
//! down the same. The sums are laid out as eight running totals side
//! by side, which the compiler turns into vector instructions. A
//! second build of them for AVX2, chosen by asking the processor, was
//! written and timed and was no faster at these lengths, so it is
//! not here (`docs/performance.md` has the numbers). What did pay was
//! doing each sum before its newest sample is written.
//!
//! `Need` is what chooses a rate when a person has not: from how hard
//! the bend is driven and how fast the sound going in moves, the
//! lowest rate that keeps what folds under `FLOOR`.

use std::f32::consts::PI;

/// The highest rate, as a power of two: sixteen times.
pub const MOST: usize = 4;
/// The loudest a folded tone may be before a higher rate is chosen,
/// against a curve that tops out at 1: 100 dB down.
pub const FLOOR: f32 = 1e-5;

/// How much of the band, as a part of the sample rate, the filters
/// leave untouched: 20 kHz at 48 kHz. Above it they start to fall.
const EDGE: f64 = 0.417;
/// How far down the filters hold what they stop: when playing, and
/// when rendering.
const PLAYING_DB: f64 = 80.0;
const RENDERING_DB: f64 = 130.0;

// The longest run of taps that are not nought, rounded up to eight.
const TAPS: usize = 56;
const ODD: usize = 32;
const PAD: usize = 128;
const SIDE: usize = 1024;
// How long one rate takes to cross into the next, in samples.
const CROSS: u32 = 32;

/// Something that bends the sound and can be told the rate it now
/// runs at, keeping what it remembers.
pub trait Bend: Copy {
    fn retune(&mut self, rate: f32);
}

// One half-band filter's taps: those either side of the middle, the
// newest sample's last, with noughts in front to make up eights.
#[derive(Clone, Copy)]
struct Taps {
    coef: [f32; TAPS],
    len: usize,
    // The middle tap is this many low-rate samples back.
    m: usize,
}

fn bessel(x: f64) -> f64 {
    let (mut sum, mut term) = (1.0, 1.0);
    for k in 1..40 {
        term *= (x / (2.0 * k as f64)).powi(2);
        sum += term;
    }
    sum
}

// A Kaiser-windowed half-band for the stage that doubles from `stage`
// doublings up: steep for the first, where the band's edge is close,
// and short after it, where there is room.
fn design(stage: usize, down_db: f64) -> Taps {
    let width = ((1 << stage) as f64 - 2.0 * EDGE) / (2 << stage) as f64;
    let wanted = (down_db - 7.95) / (14.36 * width) + 1.0;
    let m = (((wanted - 3.0) / 4.0).ceil().max(1.0) as usize).min((TAPS - 2) / 2);
    let beta = 0.1102 * (down_db - 8.7);
    let centre = (2 * m + 1) as f64;
    let side = 2 * m + 2;
    let raw: Vec<f64> = (0..side)
        .map(|k| {
            let off = 2.0 * k as f64 - centre;
            let arg = std::f64::consts::PI * off / 2.0;
            let window = bessel(beta * (1.0 - (off / (centre + 1.0)).powi(2)).sqrt());
            arg.sin() / arg * window
        })
        .collect();
    // The middle tap is a half, so the rest must come to a half for
    // a steady level to pass at its own level.
    let scale = 0.5 / raw.iter().sum::<f64>();
    let len = side.div_ceil(8) * 8;
    let mut coef = [0.0; TAPS];
    for (k, h) in raw.iter().enumerate() {
        coef[len - 1 - k] = (h * scale) as f32;
    }
    Taps { coef, len, m }
}

#[inline(always)]
fn abreast(a: &[f32], b: &[f32]) -> f32 {
    let mut acc = [0.0f32; 8];
    for (x, y) in a.chunks_exact(8).zip(b.chunks_exact(8)) {
        for i in 0..8 {
            acc[i] += x[i] * y[i];
        }
    }
    ((acc[0] + acc[4]) + (acc[1] + acc[5])) + ((acc[2] + acc[6]) + (acc[3] + acc[7]))
}

/// The sum of `a` times `b`, whose lengths are the same count of
/// eights.
#[inline]
pub fn dot(a: &[f32], b: &[f32]) -> f32 {
    abreast(a, b)
}

// A run of samples kept twice over, so the last `len` are always side
// by side for the sum.
#[derive(Clone, Copy)]
struct Kept {
    buf: [f32; 2 * TAPS],
    at: usize,
}

impl Kept {
    const fn new() -> Self {
        Self {
            buf: [0.0; 2 * TAPS],
            at: 0,
        }
    }

    // Takes `x` in and gives the sum of the last `len` samples, `x`
    // among them, each times its tap. The sum is done before `x` is
    // written, over the sample it will replace, and put right after:
    // reading eight samples at once across one written a moment ago
    // makes a processor wait, and that wait cost more than the sum.
    #[inline]
    fn sum(&mut self, x: f32, taps: &Taps) -> f32 {
        let len = taps.len;
        let next = if self.at + 1 == len { 0 } else { self.at + 1 };
        let stale = self.buf[self.at];
        let sum = dot(&self.buf[next..next + len], &taps.coef[..len]);
        self.buf[self.at] = x;
        self.buf[self.at + len] = x;
        self.at = next;
        sum + (x - stale) * taps.coef[len - 1]
    }

    // The sample taken in `back` samples before the last.
    #[inline]
    fn back(&self, back: usize, len: usize) -> f32 {
        self.buf[self.at + len - 1 - back]
    }
}

// One doubling: the way up and the way back down.
#[derive(Clone, Copy)]
struct Half {
    up: Kept,
    even: Kept,
    odd: [f32; ODD],
    odd_at: usize,
    // One more sample of delay on the way down, where the stages
    // above would otherwise leave this one half a sample out.
    pad: bool,
    held: f32,
}

impl Half {
    const fn new() -> Self {
        Self {
            up: Kept::new(),
            even: Kept::new(),
            odd: [0.0; ODD],
            odd_at: 0,
            pad: false,
            held: 0.0,
        }
    }

    // One sample in, the two that stand for it at twice the rate.
    #[inline]
    fn rise(&mut self, taps: &Taps, x: f32) -> (f32, f32) {
        let sum = self.up.sum(x, taps);
        (2.0 * sum, self.up.back(taps.m, taps.len))
    }

    // Two samples in at twice the rate, one out.
    #[inline]
    fn fall(&mut self, taps: &Taps, first: f32, second: f32) -> f32 {
        let sum = self.even.sum(first, taps);
        self.odd[self.odd_at] = second;
        let middle = self.odd[(self.odd_at + ODD - taps.m - 1) % ODD];
        self.odd_at = (self.odd_at + 1) % ODD;
        let out = sum + 0.5 * middle;
        match self.pad {
            true => std::mem::replace(&mut self.held, out),
            false => out,
        }
    }
}

// Up through each doubling in turn, the bend at the top, and down.
fn through(halves: &mut [Half], taps: &[Taps], x: f32, bend: &mut impl FnMut(f32) -> f32) -> f32 {
    let (Some((half, above)), Some((tap, later))) = (halves.split_first_mut(), taps.split_first())
    else {
        return bend(x);
    };
    let (a, b) = half.rise(tap, x);
    let (a, b) = (
        through(above, later, a, bend),
        through(above, later, b, bend),
    );
    half.fall(tap, a, b)
}

// How long each doubling of a lane to `log2` delays the sound, as its
// filters alone and with the odd samples padded out: the whole delay
// in samples at the plain rate, which doublings pad, and how late the
// sound is at the top, in samples there.
fn delays(taps: &[Taps; MOST], log2: usize) -> (usize, [bool; MOST], usize) {
    let mut pads = [false; MOST];
    let (mut below, mut at_top) = (0, 0);
    for s in (0..log2).rev() {
        let own = 2 * taps[s].m + 1;
        // What is above must come to an even count here, or halving
        // it would leave half a sample.
        pads[s] = s > 0 && (own + below / 2) % 2 == 1;
        below = own + usize::from(pads[s]) + below / 2;
        at_top += own << (log2 - 1 - s);
    }
    (below, pads, at_top)
}

// The bend at one rate, with everything it needs to stand in for
// another without a join showing.
#[derive(Clone, Copy)]
struct Lane<T> {
    log2: usize,
    halves: [Half; MOST],
    // The plain-rate delay that makes every lane as late as the
    // latest.
    pad: [f32; PAD],
    pad_at: usize,
    lag: usize,
    // The second signal, at the top rate, held back to meet the sound.
    side: [f32; SIDE],
    side_at: usize,
    side_lag: usize,
    side_was: f32,
    inner: T,
}

impl<T: Bend> Lane<T> {
    fn new(taps: &[Taps; MOST], log2: usize, latest: usize, rate: f32, mut inner: T) -> Self {
        let (delay, pads, at_top) = delays(taps, log2);
        let mut halves = [Half::new(); MOST];
        for (half, pad) in halves.iter_mut().zip(pads) {
            half.pad = pad;
        }
        inner.retune(rate * (1 << log2) as f32);
        Self {
            log2,
            halves,
            pad: [0.0; PAD],
            pad_at: 0,
            lag: latest - delay,
            side: [0.0; SIDE],
            side_at: 0,
            // Drawing a line to each new value of the second signal
            // already makes it one plain sample late.
            side_lag: at_top.saturating_sub(1 << log2),
            side_was: 0.0,
            inner,
        }
    }

    #[inline]
    fn run(
        &mut self,
        taps: &[Taps; MOST],
        x: f32,
        side: f32,
        f: &mut impl FnMut(&mut T, f32, f32) -> f32,
    ) -> f32 {
        let log2 = self.log2;
        let bent = match log2 {
            0 => f(&mut self.inner, x, side),
            _ => {
                let step = (side - self.side_was) / (1 << log2) as f32;
                let mut now = self.side_was;
                let Self {
                    halves,
                    side: ring,
                    side_at,
                    side_lag,
                    inner,
                    ..
                } = self;
                let mut bend = |x: f32| {
                    ring[*side_at] = now;
                    now += step;
                    let met = ring[(*side_at + SIDE - *side_lag) % SIDE];
                    *side_at = (*side_at + 1) % SIDE;
                    f(inner, x, met)
                };
                through(&mut halves[..log2], &taps[..log2], x, &mut bend)
            }
        };
        self.side_was = side;
        self.pad[self.pad_at] = bent;
        let out = self.pad[(self.pad_at + PAD - self.lag) % PAD];
        self.pad_at = (self.pad_at + 1) % PAD;
        out
    }
}

#[derive(Clone, Copy)]
enum Phase {
    Steady,
    // The coming lane is run and not heard, this many samples more.
    Filling(u32),
    // The two are crossed, this many samples more.
    Crossing(u32),
}

/// A bend, run at whatever rate it is asked for.
#[derive(Clone, Copy)]
pub struct Oversampled<T: Bend> {
    rate: f32,
    taps: [Taps; MOST],
    latest: usize,
    now: Lane<T>,
    coming: Lane<T>,
    phase: Phase,
}

impl<T: Bend> Oversampled<T> {
    /// `inner` as made for `rate`. `rendering` picks the steeper
    /// filters, which keep more out and are later for it. Works out
    /// its filters here, so never on the audio thread.
    pub fn new(rate: f32, rendering: bool, inner: T) -> Self {
        let down = match rendering {
            true => RENDERING_DB,
            false => PLAYING_DB,
        };
        let mut taps = [design(0, down); MOST];
        (1..MOST).for_each(|s| taps[s] = design(s, down));
        let latest = (0..=MOST).map(|k| delays(&taps, k).0).max().unwrap_or(0);
        let now = Lane::new(&taps, 0, latest, rate, inner);
        Self {
            rate,
            taps,
            latest,
            now,
            coming: now,
            phase: Phase::Steady,
        }
    }

    /// Samples the sound comes out late by, at every rate alike.
    pub fn delay(&self) -> usize {
        self.latest
    }

    /// The rate now running, as a power of two.
    pub fn log2(&self) -> usize {
        self.now.log2
    }

    /// The bend as it now runs.
    pub fn inner(&self) -> &T {
        &self.now.inner
    }

    /// Does `f` to the bend, at whichever rates it is running.
    pub fn each(&mut self, f: impl Fn(&mut T)) {
        f(&mut self.now.inner);
        f(&mut self.coming.inner);
    }

    /// Forgets the sound so far. The rate stays.
    pub fn reset(&mut self, f: impl Fn(&mut T)) {
        let inner = self.now.inner;
        self.now = Lane::new(&self.taps, self.now.log2, self.latest, self.rate, inner);
        f(&mut self.now.inner);
        self.coming = self.now;
        self.phase = Phase::Steady;
    }

    /// One sample through the bend at two to the `log2` times the
    /// rate. `f` is the bend itself: given the thing that bends, a
    /// sample, and `side` as it stands when that sample gets there.
    #[inline]
    pub fn next(
        &mut self,
        x: f32,
        side: f32,
        log2: usize,
        mut f: impl FnMut(&mut T, f32, f32) -> f32,
    ) -> f32 {
        let log2 = log2.min(MOST);
        if matches!(self.phase, Phase::Steady) && log2 != self.now.log2 {
            self.coming = Lane::new(&self.taps, log2, self.latest, self.rate, self.now.inner);
            self.phase = Phase::Filling(self.latest as u32 + 40);
        }
        let taps = &self.taps;
        let heard = self.now.run(taps, x, side, &mut f);
        match self.phase {
            Phase::Steady => heard,
            Phase::Filling(left) => {
                self.coming.run(taps, x, side, &mut f);
                self.phase = match left {
                    0 | 1 => Phase::Crossing(CROSS),
                    _ => Phase::Filling(left - 1),
                };
                heard
            }
            Phase::Crossing(left) => {
                let next = self.coming.run(taps, x, side, &mut f);
                let across = 1.0 - left as f32 / CROSS as f32;
                self.phase = match left {
                    0 | 1 => {
                        self.now = self.coming;
                        Phase::Steady
                    }
                    _ => Phase::Crossing(left - 1),
                };
                heard + (next - heard) * across
            }
        }
    }
}

/// Chooses a rate from the sound: `next` is given each sample going
/// into the bend and how far up its curve that sample reaches
/// (`f32::INFINITY` for a curve with a corner in reach), and answers
/// with the lowest rate, as a power of two, that keeps what folds
/// under `FLOOR`.
///
/// A curve as smooth as `tanh` makes harmonics that fall away by a
/// steady ratio from one to the next, and how fast depends on how far
/// up the curve the sound reaches: gently driven, each is a small
/// part of the last; driven hard, they barely fall at all. The first
/// harmonic that can fold into hearing is the one that reaches past
/// the running rate less the top of hearing, so the faster the sound
/// moves and the harder it is driven, the higher the rate has to be.
/// A rate is taken up at once and let go slowly.
#[derive(Debug, Clone, Copy)]
pub struct Need {
    rate: f32,
    reach: f32,
    size: f32,
    slope: f32,
    last: f32,
    fall: f32,
    log2: usize,
    calm: u32,
    hold: u32,
    tick: u32,
}

// How often the rate is thought about again, in samples.
const THINK: u32 = 16;

impl Need {
    pub fn new(rate: f32) -> Self {
        Self {
            rate,
            reach: 0.0,
            size: 0.0,
            slope: 0.0,
            last: 0.0,
            fall: crate::pole(50.0, rate),
            log2: 0,
            calm: 0,
            hold: (rate * 0.25) as u32,
            tick: 0,
        }
    }

    pub fn reset(&mut self) {
        *self = Self::new(self.rate);
    }

    /// The lowest rate that would do for a sound reaching `reach` up
    /// the curve whose fastest part moves like a sine at `hz`.
    pub fn wanted(rate: f32, reach: f32, hz: f32) -> usize {
        if !reach.is_finite() {
            return MOST;
        }
        if reach <= 0.0 || hz <= 0.0 {
            return 0;
        }
        // How fast the harmonics fall, and so how many it takes to
        // get under the floor: `tanh` stops being smooth a quarter
        // turn up the imaginary axis, and that is what sets it.
        let falls = (PI / (2.0 * reach)).asinh();
        let orders = (1.0 / FLOOR).ln() / falls;
        let heard = (rate * 0.45).min(20_000.0);
        let room = orders * hz + heard;
        (0..MOST)
            .find(|k| rate * (1 << k) as f32 >= room)
            .unwrap_or(MOST)
    }

    #[inline]
    pub fn next(&mut self, x: f32, reach: f32) -> usize {
        self.reach = reach.max(self.reach * self.fall);
        self.size = x.abs().max(self.size * self.fall);
        self.slope = (x - self.last).abs().max(self.slope * self.fall);
        self.last = x;
        self.tick += 1;
        if self.tick < THINK {
            return self.log2;
        }
        self.tick = 0;
        let want = match self.size > 1e-9 {
            // A sine's biggest step is its size times twice the sine
            // of half the angle it turns in a sample.
            true => {
                let turn = (self.slope / (2.0 * self.size)).min(1.0).asin();
                Self::wanted(self.rate, self.reach, self.rate / PI * turn)
            }
            false => 0,
        };
        if want > self.log2 {
            self.log2 = want;
            self.calm = 0;
        } else if want < self.log2 {
            self.calm += THINK;
            if self.calm >= self.hold {
                self.log2 -= 1;
                self.calm = 0;
            }
        } else {
            self.calm = 0;
        }
        self.log2
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const RATE: f32 = 48_000.0;

    // Nothing at all: what goes in comes out.
    #[derive(Clone, Copy)]
    struct Wire;
    impl Bend for Wire {
        fn retune(&mut self, _rate: f32) {}
    }

    fn sine(hz: f32, n: usize) -> Vec<f32> {
        // Worked out in doubles: a float loses count of the angle long
        // before the end, and that would be heard as noise.
        (0..n)
            .map(|i| (std::f64::consts::TAU * hz as f64 * i as f64 / RATE as f64).sin() as f32)
            .collect()
    }

    // How far `out` is from `input` made `delay` late, at worst, once
    // the first `skip` samples are past.
    fn apart(input: &[f32], out: &[f32], delay: usize, skip: usize) -> f32 {
        (skip..out.len())
            .map(|i| (out[i] - input[i - delay]).abs())
            .fold(0.0, f32::max)
    }

    // How much of `hz` is in `x`, as the size of a sine.
    fn part(x: &[f32], hz: f32) -> f32 {
        let (mut re, mut im) = (0.0f64, 0.0f64);
        for (i, v) in x.iter().enumerate() {
            let turn = 2.0 * std::f64::consts::PI * hz as f64 * i as f64 / RATE as f64;
            // A Hann window, so a tone between bins does not smear.
            let w = 0.5 - 0.5 * (2.0 * std::f64::consts::PI * i as f64 / x.len() as f64).cos();
            re += *v as f64 * w * turn.cos();
            im += *v as f64 * w * turn.sin();
        }
        (4.0 * (re * re + im * im).sqrt() / x.len() as f64) as f32
    }

    #[test]
    fn every_rate_is_late_by_the_same_whole_number_of_samples() {
        let input = sine(1000.0, 6000);
        for rendering in [false, true] {
            // What the filters leave of a tone inside the band.
            let near = if rendering { 2e-5 } else { 2e-3 };
            for log2 in 0..=MOST {
                let mut over = Oversampled::new(RATE, rendering, Wire);
                let out: Vec<f32> = input
                    .iter()
                    .map(|x| over.next(*x, 0.0, log2, |_, x, _| x))
                    .collect();
                let off = apart(&input, &out, over.delay(), 2000);
                assert!(off < near, "rendering {rendering}, log2 {log2}: {off}");
            }
        }
    }

    #[test]
    fn the_delay_is_short_when_playing_and_longer_when_rendering() {
        let playing = Oversampled::new(RATE, false, Wire).delay();
        let rendering = Oversampled::new(RATE, true, Wire).delay();
        assert!(playing < 48, "playing: {playing}");
        assert!(
            rendering > playing && rendering < PAD,
            "rendering: {rendering}"
        );
    }

    #[test]
    fn a_second_signal_arrives_with_the_sound() {
        // The bend gives back the second signal alone, so what comes
        // out shows when it got there.
        let side = sine(300.0, 6000);
        for log2 in 0..=MOST {
            let mut over = Oversampled::new(RATE, true, Wire);
            let out: Vec<f32> = side
                .iter()
                .map(|s| over.next(0.0, *s, log2, |_, _, side| side))
                .collect();
            let off = apart(&side, &out, over.delay(), 2000);
            assert!(off < 2e-3, "log2 {log2}: {off}");
        }
    }

    #[test]
    fn changing_rate_is_not_heard() {
        let input = sine(1000.0, 32_000);
        let mut over = Oversampled::new(RATE, false, Wire);
        let rates = [0, 3, 1, 4, 2, 0, 4, 0];
        let out: Vec<f32> = input
            .iter()
            .enumerate()
            .map(|(i, x)| over.next(*x, 0.0, rates[(i / 4000) % rates.len()], |_, x, _| x))
            .collect();
        let off = apart(&input, &out, over.delay(), 2000);
        assert!(off < 2e-3, "{off}");
        assert_eq!(over.log2(), 0);
    }

    // A sine at 5 kHz driven four times over into `tanh`: its ninth
    // harmonic is at 45 kHz, which at the plain rate folds to 3 kHz.
    fn folded(log2: usize, rendering: bool) -> f32 {
        let input = sine(5000.0, 48_000);
        let mut over = Oversampled::new(RATE, rendering, Wire);
        let out: Vec<f32> = input
            .iter()
            .map(|x| over.next(*x * 4.0, 0.0, log2, |_, x, _| x.tanh()))
            .collect();
        part(&out[8000..], 3000.0)
    }

    #[test]
    fn what_folds_at_the_plain_rate_is_gone_at_a_higher_one() {
        let plain = folded(0, true);
        assert!(plain > 1e-2, "the test has nothing to find: {plain}");
        // The ninth is clear at four times; what is left there is
        // the twenty-ninth and up.
        let cases = [(2, false, 3e-4), (3, false, 1e-4), (4, false, 1e-4)];
        // With the steeper filters what is left is no longer folding
        // but the last of the filters' own images, beating with the
        // harmonics: under the floor all the same.
        let fine = [(2, true, 3e-4), (3, true, FLOOR), (4, true, FLOOR)];
        for (log2, rendering, most) in cases.into_iter().chain(fine) {
            let left = folded(log2, rendering);
            assert!(left < most, "log2 {log2}, rendering {rendering}: {left}");
        }
    }

    #[test]
    fn the_sum_abreast_is_the_sum() {
        let a: Vec<f32> = (0..TAPS).map(|i| (i as f32 * 0.37).sin()).collect();
        let b: Vec<f32> = (0..TAPS).map(|i| (i as f32 * 1.91).cos()).collect();
        let straight: f32 = a.iter().zip(&b).map(|(x, y)| x * y).sum();
        assert!((dot(&a, &b) - straight).abs() < 1e-4);
    }

    // What a rate leaves that the highest does not: the folded part.
    fn left_by(log2: usize, hz: f32, drive: f32) -> f32 {
        let input = sine(hz, 24_000);
        let run = |log2: usize| -> Vec<f32> {
            let mut over = Oversampled::new(RATE, true, Wire);
            input
                .iter()
                .map(|x| over.next(*x * drive, 0.0, log2, |_, x, _| x.tanh()))
                .collect()
        };
        let (got, best) = (run(log2), run(MOST));
        let sum: f32 = (4000..got.len()).map(|i| (got[i] - best[i]).powi(2)).sum();
        (sum / (got.len() - 4000) as f32).sqrt()
    }

    #[test]
    fn the_rate_chosen_keeps_what_folds_under_the_floor() {
        for hz in [100.0, 1000.0, 3000.0, 7000.0, 12_000.0, 17_000.0] {
            for drive in [0.05, 0.3, 1.0, 3.0, 8.0] {
                let log2 = Need::wanted(RATE, drive, hz);
                let left = left_by(log2, hz, drive);
                assert!(
                    left < 3.0 * FLOOR,
                    "{hz} Hz driven {drive}: chose {log2}, left {left}"
                );
            }
        }
    }

    #[test]
    fn nothing_is_spent_where_it_adds_nothing() {
        // A bass driven hard, and anything at all barely driven.
        let cases = [(60.0, 8.0), (200.0, 3.0), (17_000.0, 0.002), (1000.0, 0.1)];
        for (hz, drive) in cases {
            assert_eq!(Need::wanted(RATE, drive, hz), 0, "{hz} Hz driven {drive}");
        }
        // And the plain rate there is as good as the highest.
        assert!(left_by(0, 200.0, 3.0) < FLOOR);
    }

    #[test]
    fn a_rate_is_taken_up_at_once_and_let_go_slowly() {
        let mut need = Need::new(RATE);
        let bright = sine(9000.0, 4800);
        let dull = sine(80.0, 48_000);
        let up = bright
            .iter()
            .map(|x| need.next(*x * 4.0, 4.0))
            .max()
            .unwrap();
        let after_16: Vec<usize> = dull.iter().map(|x| need.next(*x * 4.0, 4.0)).collect();
        assert!(up >= 3, "a bright sound driven hard chose {up}");
        assert_eq!(after_16[2000], up, "let go within 40 ms");
        assert_eq!(*after_16.last().unwrap(), 0);
    }

    #[test]
    fn a_corner_in_reach_asks_for_the_most() {
        assert_eq!(Need::wanted(RATE, f32::INFINITY, 50.0), MOST);
    }

    use crate::{IronShape, PENTODE, STRAPPED, TRIODE, Transformer, Tube, TubeShape};

    // A valve set as `shape`, a sine at `hz` driven by `drive`, at
    // the rate `log2` or, with `None`, whatever `Need` chooses.
    fn valve(shape: TubeShape, hz: f32, drive: f32, log2: Option<usize>) -> (Vec<f32>, usize) {
        let mut tube = Tube::new(RATE);
        tube.set(shape);
        let mut over = Oversampled::new(RATE, true, tube);
        let mut need = Need::new(RATE);
        let mut top = 0;
        let out = sine(hz, 24_000)
            .iter()
            .map(|x| {
                let x = x * drive;
                let chosen = need.next(x, tube.reach(x, 0.0));
                top = top.max(chosen);
                over.next(x, 0.0, log2.unwrap_or(chosen), |t, x, s| t.next(x, s))
            })
            .collect();
        (out, top)
    }

    #[test]
    fn the_rate_chosen_for_a_valve_keeps_what_folds_under_the_floor() {
        let shapes = [
            ("at rest", TubeShape::default()),
            (
                "a triode",
                TubeShape {
                    stage: TRIODE,
                    ..TubeShape::default()
                },
            ),
            (
                "strapped, with feedback",
                TubeShape {
                    stage: STRAPPED,
                    feedback: 0.6,
                    ..TubeShape::default()
                },
            ),
            (
                "a hard grid, biased, glowing",
                TubeShape {
                    stage: PENTODE,
                    grid: 0.8,
                    bias: 0.4,
                    glow: 0.7,
                    ..TubeShape::default()
                },
            ),
        ];
        for (name, shape) in shapes {
            for hz in [175.0, 2025.0, 9025.0, 16_025.0] {
                for drive in [0.1, 0.7, 3.0] {
                    let (got, chose) = valve(shape, hz, drive, None);
                    let left = strays(&got, hz);
                    // Where even the most is asked for, the most is
                    // what there is: a hard grid lifted and driven
                    // at the very top still leaves 80 dB down.
                    let allowed = if chose == MOST { 30.0 } else { 3.0 };
                    assert!(
                        left < allowed * FLOOR,
                        "{name}, {hz} Hz driven {drive}: chose {chose}, left {left}"
                    );
                }
            }
        }
        // And the test can tell: the same, held at the plain rate.
        let (plain, _) = valve(TubeShape::default(), 9025.0, 3.0, Some(0));
        assert!(strays(&plain, 9025.0) > 100.0 * FLOOR);
    }

    // What is in the last 1920 samples of `x` that can be heard and is
    // not `hz` or a harmonic of it. Every multiple of 25 Hz fits that
    // many samples a whole number of times, so a tone at one can be
    // taken out exactly, and what folds lands between its harmonics.
    // What lies above 20 kHz is taken out too: the filters are made
    // to let the fold fall there, where it is not heard.
    fn strays(x: &[f32], hz: f32) -> f32 {
        let tail = &x[x.len() - 1920..];
        let mut left: Vec<f64> = tail.iter().map(|v| *v as f64).collect();
        let mean = left.iter().sum::<f64>() / 1920.0;
        left.iter_mut().for_each(|v| *v -= mean);
        let harmonics = (1..)
            .map(|n| n as f64 * hz as f64)
            .take_while(|f| *f < 20_000.0);
        let above = (800..960).map(|bin| bin as f64 * 25.0);
        for tone in harmonics.chain(above) {
            let turn = |i: usize| std::f64::consts::TAU * tone * i as f64 / RATE as f64;
            let re = (0..1920).map(|i| left[i] * turn(i).cos()).sum::<f64>() / 960.0;
            let im = (0..1920).map(|i| left[i] * turn(i).sin()).sum::<f64>() / 960.0;
            (0..1920).for_each(|i| left[i] -= re * turn(i).cos() + im * turn(i).sin());
        }
        // And the very top, which has a cosine and no sine.
        let top = (0..1920)
            .map(|i| left[i] * if i % 2 == 0 { 1.0 } else { -1.0 })
            .sum::<f64>()
            / 1920.0;
        (0..1920).for_each(|i| left[i] -= top * if i % 2 == 0 { 1.0 } else { -1.0 });
        (left.iter().map(|v| v * v).sum::<f64>() / 1920.0).sqrt() as f32
    }

    #[test]
    fn a_valve_at_rest_fed_a_gentle_bass_runs_at_the_plain_rate() {
        let (_, chose) = valve(TubeShape::default(), 80.0, 2.0, None);
        assert_eq!(chose, 0);
    }

    #[test]
    fn a_core_sounds_the_same_at_any_rate_where_nothing_folds() {
        // A bass into a driven core: its harmonics are far below any
        // rate's half, so every rate should give the same sound.
        let shape = IronShape {
            drive: 0.7,
            iron: 0.5,
            ..IronShape::default()
        };
        let run = |log2: usize| -> Vec<f32> {
            let mut core = Transformer::new(RATE);
            core.set(shape);
            let mut over = Oversampled::new(RATE, true, core);
            over.each(|c| c.set(shape));
            sine(70.0, 24_000)
                .iter()
                .map(|x| over.next(x * 0.8, 0.0, log2, |c, x, _| c.next(x)))
                .collect()
        };
        let plain = run(0);
        assert!(
            apart(&sine(70.0, 24_000), &plain, 0, 12_000) > 1e-3,
            "nothing bent"
        );
        for log2 in 1..=MOST {
            let got = run(log2);
            let off = (12_000..got.len())
                .map(|i| (got[i] - plain[i]).abs())
                .fold(0.0, f32::max);
            assert!(off < 2e-3, "log2 {log2}: {off}");
        }
    }

    // Not a test of anything: prints what each rate costs, for
    // `docs/performance.md`.
    // `cargo test --release -p kit-dsp what_each_rate_costs -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn what_each_rate_costs() {
        let input = sine(1000.0, 480_000);
        for rendering in [false, true] {
            for log2 in 0..=MOST {
                let cost = |valve: bool| {
                    let mut over = Oversampled::new(RATE, rendering, Tube::new(RATE));
                    let began = std::time::Instant::now();
                    let sum: f32 = input
                        .iter()
                        .map(|x| {
                            over.next(*x, 0.0, log2, |t, x, s| match valve {
                                true => t.next(x, s),
                                false => x,
                            })
                        })
                        .sum();
                    std::hint::black_box(sum);
                    began.elapsed().as_secs_f64() / 10.0 * 100.0
                };
                println!(
                    "rendering {rendering}, x{}: filters {:.3}% of a core; with a valve {:.3}%",
                    1 << log2,
                    cost(false),
                    cost(true)
                );
            }
        }
    }
}
