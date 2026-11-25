//! What Undertow itself promises: written before the code that keeps
//! each promise.

use kit::{Param, Plugin, Runner};
use kit_test::signals;
use undertow::{DRIVE, FOLLOW, LEAN, MIX, OUTPUT, PUSH, SIDE_ON_MID, STEER, Undertow, leaned};

const RATE: f32 = 48_000.0;
const SECOND: usize = RATE as usize;

// A runner with each named setting at a value in its own terms.
fn with(settings: &[(usize, f64)]) -> Runner<Undertow> {
    let mut runner = Runner::<Undertow>::new(RATE, 256);
    for &(index, plain) in settings {
        let p: &Param = &Undertow::PARAMS[index];
        runner.params[index] = p.normalized(plain);
    }
    runner
}

// Left and right made from a middle and a side.
fn stereo(mid: &[f32], side: &[f32]) -> Vec<Vec<f32>> {
    let left = mid.iter().zip(side).map(|(m, s)| m + s).collect();
    let right = mid.iter().zip(side).map(|(m, s)| m - s).collect();
    vec![left, right]
}

// The middle and the side of what came out, once the settings have
// been eased to and the filters have settled.
fn settled(runner: &mut Runner<Undertow>, input: &[Vec<f32>]) -> (Vec<f32>, Vec<f32>) {
    let lead: Vec<Vec<f32>> = input
        .iter()
        .map(|ch| (0..SECOND / 2).map(|i| ch[i % ch.len()]).collect())
        .collect();
    runner.render(&lead, lead[0].len(), 256);
    let out = runner.render(input, input[0].len(), 256);
    let mid = out[0].iter().zip(&out[1]).map(|(l, r)| (l + r) / 2.0);
    let side = out[0].iter().zip(&out[1]).map(|(l, r)| (l - r) / 2.0);
    (mid.collect(), side.collect())
}

fn worst(a: &[f32], b: &[f32]) -> f32 {
    a.iter()
        .zip(b)
        .map(|(x, y)| (x - y).abs())
        .fold(0.0, f32::max)
}

// The part of `out` that is not a scaled copy of `input`: what the
// curve added or took away beyond a change of level.
fn residual(out: &[f32], input: &[f32]) -> f32 {
    let dot: f32 = out.iter().zip(input).map(|(o, i)| o * i).sum();
    let own: f32 = input.iter().map(|i| i * i).sum();
    let scale = dot / own;
    let left: f32 = out
        .iter()
        .zip(input)
        .map(|(o, i)| (o - i * scale).powi(2))
        .sum();
    (left / out.len() as f32).sqrt()
}

// The component of `x` at `hz`, as a complex number.
fn at(x: &[f32], hz: f32) -> (f32, f32) {
    let w = std::f32::consts::TAU * hz / RATE;
    let (re, im) = x.iter().enumerate().fold((0.0, 0.0), |(re, im), (n, s)| {
        let p = w * n as f32;
        (re + s * p.cos(), im - s * p.sin())
    });
    (re / x.len() as f32, im / x.len() as f32)
}

// How much of `x` lies within 100 Hz of `hz`, in 10 Hz steps.
fn near(x: &[f32], hz: f32) -> f32 {
    (-10..=10)
        .map(|k| {
            let (re, im) = at(x, hz + k as f32 * 10.0);
            re * re + im * im
        })
        .sum::<f32>()
        .sqrt()
}

#[test]
fn the_curve_at_rest_keeps_a_small_sound_and_silence_stays_silent() {
    let cases = [
        // (sample, drive, bias, want, within)
        (0.01, 1.0, 0.0, 0.01, 1e-5),
        (0.01, 4.0, 0.0, 0.01, 1e-4),
        (-0.01, 4.0, 0.0, -0.01, 1e-4),
        (0.0, 4.0, 1.5, 0.0, 0.0),
        (0.0, 1.0, -2.0, 0.0, 0.0),
        // Off centre the curve is flatter, so the same sound is smaller.
        (0.01, 1.0, 1.0, 0.0042, 1e-3),
        (0.01, 1.0, -1.0, 0.0042, 1e-3),
    ];

    for (x, drive, bias, want, within) in cases {
        let got = leaned(x, drive, bias);
        assert!(
            (got - want).abs() <= within,
            "{x} at drive {drive}, bias {bias}: {got}"
        );
    }
}

#[test]
fn a_mono_sound_comes_out_exactly_as_it_went_in() {
    let voice = signals::sung(RATE);
    let mut runner = with(&[(PUSH, 100.0), (DRIVE, 24.0)]);
    let input = vec![voice.clone(), voice.clone()];
    let lead = vec![vec![0.0; SECOND / 2]; 2];
    runner.render(&lead, lead[0].len(), 256);
    let out = runner.render(&input, voice.len(), 256);

    assert!(
        worst(&out[0], &voice) < 1e-6,
        "left off by {}",
        worst(&out[0], &voice)
    );
    assert!(
        worst(&out[1], &voice) < 1e-6,
        "right off by {}",
        worst(&out[1], &voice)
    );
}

#[test]
fn the_middle_passes_untouched_and_the_sides_are_changed() {
    let bass = signals::sine(RATE, 1.5, 55.0, 0.5);
    let voice = signals::sung(RATE);
    let mid: Vec<f32> = bass.iter().zip(&voice).map(|(b, v)| b + v * 0.3).collect();
    let side = signals::sine(RATE, 1.5, 1000.0, 0.2);
    let input = stereo(&mid, &side);
    let mut runner = with(&[(PUSH, 100.0), (DRIVE, 12.0)]);
    let (out_mid, out_side) = settled(&mut runner, &input);

    assert!(
        worst(&out_mid, &mid) < 1e-5,
        "mid off by {}",
        worst(&out_mid, &mid)
    );
    let changed = residual(&out_side, &side);
    assert!(changed > 0.01, "the side was barely changed: {changed}");
}

#[test]
fn the_bass_brings_even_harmonics_the_curve_at_rest_does_not_make() {
    let mid = signals::sine(RATE, 1.5, 40.0, 0.6);
    let side = signals::sine(RATE, 1.5, 1000.0, 0.2);
    let input = stereo(&mid, &side);
    let second = |push: f64| {
        let mut runner = with(&[(PUSH, push), (DRIVE, 6.0), (FOLLOW, 160.0)]);
        let (_, out_side) = settled(&mut runner, &input);
        near(&out_side[..SECOND], 2000.0)
    };
    let (rest, pushed) = (second(0.0), second(100.0));

    assert!(rest < 1e-4, "at rest there is a second harmonic: {rest}");
    // Spread into sidebands round 2 kHz by the bass, so each is small.
    assert!(
        pushed > 0.002,
        "pushed, the second harmonic is only {pushed}"
    );
}

#[test]
fn the_two_halves_of_the_bass_push_the_sides_opposite_ways() {
    // A middle held above and then below the centre: an envelope of
    // it would be the same both times, the wave is not.
    let side = signals::sine(RATE, 1.5, 1000.0, 0.2);
    let second = |level: f32| {
        let mid = vec![level; side.len()];
        let mut runner = with(&[(PUSH, 60.0), (DRIVE, 6.0)]);
        let (_, out_side) = settled(&mut runner, &stereo(&mid, &side));
        at(&out_side[..SECOND], 2000.0)
    };
    let (up, down) = (second(0.4), second(-0.4));
    let dot = up.0 * down.0 + up.1 * down.1;
    let (a, b) = (up.0.hypot(up.1), down.0.hypot(down.1));

    assert!(a > 0.002 && b > 0.002, "second harmonics {a} and {b}");
    assert!(dot < -0.9 * a * b, "not opposite: {up:?} against {down:?}");
}

#[test]
fn only_the_low_end_below_follow_does_the_pushing() {
    let mid = signals::sine(RATE, 1.5, 700.0, 0.5);
    let side = signals::sine(RATE, 1.5, 3100.0, 0.1);
    let input = stereo(&mid, &side);
    let change = |follow: f64| {
        let mut runner = with(&[(PUSH, 100.0), (FOLLOW, follow), (DRIVE, 0.0)]);
        let (_, out_side) = settled(&mut runner, &input);
        residual(&out_side, &side)
    };
    let (low, high) = (change(30.0), change(1000.0));

    assert!(
        high > low * 10.0,
        "follow 30 Hz: {low}, follow 1000 Hz: {high}"
    );
}

#[test]
fn steered_the_other_way_the_sides_pass_and_the_middle_leans() {
    let mid = signals::sine(RATE, 1.5, 1000.0, 0.2);
    let side = signals::sine(RATE, 1.5, 45.0, 0.5);
    let input = stereo(&mid, &side);
    let mut runner = with(&[(STEER, SIDE_ON_MID as f64), (PUSH, 100.0), (DRIVE, 6.0)]);
    let (out_mid, out_side) = settled(&mut runner, &input);

    assert!(
        worst(&out_side, &side) < 1e-5,
        "side off by {}",
        worst(&out_side, &side)
    );
    let changed = residual(&out_mid, &mid);
    assert!(changed > 0.01, "the middle was barely changed: {changed}");
}

#[test]
fn with_no_mix_the_sound_is_the_input() {
    let mid = signals::sine(RATE, 1.0, 60.0, 0.5);
    let side = signals::white(RATE, 1.0)
        .iter()
        .map(|s| s * 0.2)
        .collect::<Vec<_>>();
    let input = stereo(&mid, &side);
    let mut runner = with(&[(MIX, 0.0), (PUSH, 100.0), (DRIVE, 24.0)]);
    let (out_mid, out_side) = settled(&mut runner, &input);

    assert!(worst(&out_mid, &mid) < 1e-5);
    assert!(
        worst(&out_side, &side) < 1e-5,
        "side off by {}",
        worst(&out_side, &side)
    );
}

#[test]
fn the_lean_meter_shows_how_far_the_bass_has_pushed() {
    let mid = signals::sine(RATE, 1.0, 50.0, 0.5);
    let side = signals::sine(RATE, 1.0, 1000.0, 0.1);
    let lean = |push: f64| {
        let mut runner = with(&[(PUSH, push)]);
        settled(&mut runner, &stereo(&mid, &side));
        runner.meters()[LEAN]
    };

    assert!(lean(0.0) < 0.01, "at no push the meter reads {}", lean(0.0));
    assert!(
        lean(100.0) > 0.3,
        "at full push the meter reads {}",
        lean(100.0)
    );
}

#[test]
fn a_moved_setting_is_eased_not_jumped() {
    let mid = vec![0.3; 4800];
    let side = signals::sine(RATE, 0.1, 500.0, 0.2);
    let input = stereo(&mid, &side);
    let mut runner = with(&[(PUSH, 0.0), (OUTPUT, 0.0)]);
    settled(&mut runner, &input);
    runner.params[PUSH] = Undertow::PARAMS[PUSH].normalized(100.0);
    runner.params[OUTPUT] = Undertow::PARAMS[OUTPUT].normalized(-24.0);
    let out = runner.render(&input, 4800, 64).remove(0);
    let step = out
        .windows(2)
        .map(|w| (w[0] - w[1]).abs())
        .fold(0.0f32, f32::max);

    // A 500 Hz sine of 0.5 at most moves about 0.033 a sample.
    assert!(step < 0.05, "the biggest step between samples was {step}");
}
