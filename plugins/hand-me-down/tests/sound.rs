//! What Hand-Me-Down itself promises: written before the code that
//! keeps each promise.

use hand_me_down::{
    BACK, BODY_GAIN, HIT_GAIN, HandMeDown, INTO, KEY, LISTEN, MATCH, MID, MIX, OUTPUT, SIDE,
    hit_weight,
};
use kit::{Param, Plugin, Runner};

const RATE: f32 = 48_000.0;
const SECOND: usize = RATE as usize;

// A runner with each named setting at a value in its own terms.
fn with(settings: &[(usize, f64)]) -> Runner<HandMeDown> {
    let mut runner = Runner::<HandMeDown>::new(RATE, 256);
    for &(index, plain) in settings {
        let p: &Param = &HandMeDown::PARAMS[index];
        runner.params[index] = p.normalized(plain);
    }
    runner
}

fn s(seconds: f32) -> usize {
    (seconds * RATE) as usize
}

// A plucked line, one note at a time: each `(start, hz, amp)` rings
// until the next one starts, and the last for `tail` seconds.
fn line(notes: &[(f32, f32, f32)], tail: f32) -> Vec<f32> {
    let end = notes.last().map_or(0.0, |n| n.0) + tail;
    let mut out = vec![0.0; s(end)];
    for (k, &(start, hz, amp)) in notes.iter().enumerate() {
        let stop = notes.get(k + 1).map_or(end, |n| n.0);
        for (n, x) in out.iter_mut().enumerate().take(s(stop)).skip(s(start)) {
            let t = (n - s(start)) as f32 / RATE;
            *x = amp * (-t / 0.2).exp() * (std::f32::consts::TAU * hz * t).sin();
        }
    }
    out
}

// What comes out for `input` (one buffer a channel), after half a
// second of silence for any moved level to be eased to. Silence
// starts no note.
fn through(runner: &mut Runner<HandMeDown>, input: &[Vec<f32>]) -> Vec<Vec<f32>> {
    let lead = vec![vec![0.0; SECOND / 2]; input.len()];
    runner.render(&lead, lead[0].len(), 256);
    runner.render(input, input[0].len(), 256)
}

fn mono(runner: &mut Runner<HandMeDown>, input: &[f32]) -> Vec<f32> {
    through(runner, &[input.to_vec(), input.to_vec()]).remove(0)
}

fn worst(a: &[f32], b: &[f32]) -> f32 {
    a.iter()
        .zip(b)
        .map(|(x, y)| (x - y).abs())
        .fold(0.0, f32::max)
}

fn rms(x: &[f32]) -> f32 {
    (x.iter().map(|v| v * v).sum::<f32>() / x.len().max(1) as f32).sqrt()
}

// The power of `x` at `hz`.
fn at(x: &[f32], hz: f32) -> f32 {
    let w = std::f32::consts::TAU * hz / RATE;
    let (re, im) = x.iter().enumerate().fold((0.0, 0.0), |(re, im), (n, v)| {
        let p = w * n as f32;
        (re + v * p.cos(), im - v * p.sin())
    });
    (re * re + im * im) / (x.len() as f32).powi(2)
}

// How much of `x` lies within 50 Hz of `hz`.
fn near(x: &[f32], hz: f32) -> f32 {
    (-5..=5).map(|k| at(x, hz + k as f32 * 10.0)).sum()
}

fn span(x: &[f32], from: f32, to: f32) -> &[f32] {
    &x[s(from)..s(to)]
}

#[test]
fn the_hit_rises_holds_and_hands_over() {
    // (samples in, crossfade, hit, fade, weight)
    let cases = [
        (0.0, 48.0, 1440.0, 960.0, 0.0),
        (24.0, 48.0, 1440.0, 960.0, 0.5),
        (48.0, 48.0, 1440.0, 960.0, 1.0),
        (1439.0, 48.0, 1440.0, 960.0, 1.0),
        (1920.0, 48.0, 1440.0, 960.0, 0.5),
        (2400.0, 48.0, 1440.0, 960.0, 0.0),
        (9000.0, 48.0, 1440.0, 960.0, 0.0),
    ];

    for (i, cross, hit, fade, want) in cases {
        let got = hit_weight(i, cross, hit, fade);
        assert!((got - want).abs() < 1e-3, "{i} in: {got}, not {want}");
    }
}

#[test]
fn with_no_mix_it_is_the_sound_that_came_in() {
    let input = line(&[(0.0, 440.0, 0.5), (0.3, 660.0, 0.5)], 0.4);
    let out = mono(&mut with(&[(MIX, 0.0)]), &input);

    assert!(worst(&out, &input) < 1e-4, "{}", worst(&out, &input));
}

#[test]
fn a_single_note_keeps_its_own_body_since_nothing_came_before() {
    let input = line(&[(0.0, 440.0, 0.5)], 0.8);
    let out = mono(&mut with(&[]), &input);

    assert!(worst(&out, &input) < 1e-4, "{}", worst(&out, &input));
}

#[test]
fn a_later_note_rings_at_the_earlier_notes_pitch() {
    let input = line(&[(0.0, 440.0, 0.5), (0.3, 660.0, 0.5)], 0.4);
    let out = mono(&mut with(&[]), &input);
    let body = span(&out, 0.4, 0.58);
    let (old, own) = (near(body, 440.0), near(body, 660.0));

    assert!(old > own * 20.0, "440 Hz {old}, 660 Hz {own}");
    // Left alone the second note's body is its own.
    let dry = span(&input, 0.4, 0.58);
    assert!(near(dry, 660.0) > near(dry, 440.0) * 20.0);
}

#[test]
fn but_its_hit_is_its_own() {
    let input = line(&[(0.0, 440.0, 0.5), (0.3, 660.0, 0.5)], 0.4);
    let out = mono(&mut with(&[]), &input);
    let hit = span(&out, 0.301, 0.326);
    let (old, own) = (near(hit, 440.0), near(hit, 660.0));

    assert!(own > old * 4.0, "660 Hz {own}, 440 Hz {old}");
}

#[test]
fn back_reaches_further_down_the_line() {
    let notes = [(0.0, 440.0, 0.5), (0.3, 587.0, 0.5), (0.6, 784.0, 0.5)];
    let input = line(&notes, 0.3);
    let body = |back: f64| {
        let out = mono(&mut with(&[(BACK, back)]), &input);
        let b = span(&out, 0.7, 0.88).to_vec();
        [near(&b, 440.0), near(&b, 587.0), near(&b, 784.0)]
    };
    let one = body(0.0);
    let two = body(1.0);

    // One back, the third note sings at the second's pitch; two back,
    // at the first's.
    assert!(one[1] > one[0].max(one[2]) * 10.0, "{one:?}");
    assert!(two[0] > two[1].max(two[2]) * 10.0, "{two:?}");
}

#[test]
fn when_the_earlier_note_runs_out_the_note_gets_its_own_body_back() {
    // A short note and then a long one: the short one has no more to
    // hand down after its own length. The second is struck harder, as
    // the first is still ringing loud when it comes.
    let input = line(&[(0.0, 440.0, 0.5), (0.15, 660.0, 0.9)], 0.6);
    let out = mono(&mut with(&[]), &input);
    let early = span(&out, 0.2, 0.28);
    let late = span(&out, 0.35, 0.65);

    assert!(near(early, 440.0) > near(early, 660.0) * 10.0);
    assert!(near(late, 660.0) > near(late, 440.0) * 10.0);
}

#[test]
fn into_the_sides_leaves_a_mono_sound_as_it_was() {
    let input = line(&[(0.0, 440.0, 0.5), (0.3, 660.0, 0.5)], 0.4);
    let out = mono(&mut with(&[(INTO, SIDE as f64)]), &input);

    assert!(worst(&out, &input) < 1e-4, "{}", worst(&out, &input));
}

#[test]
fn into_the_middle_hands_down_the_middle_and_keeps_the_sides_own() {
    let mid = line(&[(0.0, 440.0, 0.4), (0.3, 660.0, 0.4)], 0.4);
    let side = line(&[(0.0, 880.0, 0.1), (0.3, 1320.0, 0.1)], 0.4);
    let left = mid.iter().zip(&side).map(|(m, s)| m + s).collect();
    let right = mid.iter().zip(&side).map(|(m, s)| m - s).collect();
    let out = through(&mut with(&[(INTO, MID as f64)]), &[left, right]);
    let half = |sign: f32| -> Vec<f32> {
        out[0]
            .iter()
            .zip(&out[1])
            .map(|(l, r)| (l + sign * r) / 2.0)
            .collect()
    };
    let (m, sd) = (half(1.0), half(-1.0));
    let (m, sd) = (span(&m, 0.4, 0.58), span(&sd, 0.4, 0.58));

    assert!(
        near(m, 440.0) > near(m, 660.0) * 20.0,
        "the middle is handed down"
    );
    assert!(
        near(sd, 1320.0) > near(sd, 880.0) * 20.0,
        "the sides are their own"
    );
}

#[test]
fn hit_and_body_levels_turn_each_up_or_down_alone() {
    let input = line(&[(0.0, 440.0, 0.5), (0.3, 660.0, 0.5)], 0.4);
    let plain = mono(&mut with(&[]), &input);
    let thin = mono(&mut with(&[(BODY_GAIN, -24.0)]), &input);
    let soft = mono(&mut with(&[(HIT_GAIN, -24.0)]), &input);
    let (hit, body) = ((0.302, 0.325), (0.4, 0.58));
    let level = |x: &[f32], (a, b): (f32, f32)| rms(span(x, a, b));

    assert!(level(&thin, body) < level(&plain, body) * 0.1);
    assert!(level(&thin, hit) > level(&plain, hit) * 0.8);
    assert!(level(&soft, hit) < level(&plain, hit) * 0.2);
    assert!(level(&soft, body) > level(&plain, body) * 0.9);
}

#[test]
fn matched_it_goes_out_as_loud_as_it_came_in() {
    let notes: Vec<(f32, f32, f32)> = (0..12)
        .map(|k| (k as f32 * 0.25, [440.0, 554.0, 660.0][k % 3], 0.5))
        .collect();
    let input = line(&notes, 0.25);
    let last = |x: &[f32]| rms(&x[x.len() - SECOND..]);
    let loud = [(BODY_GAIN, 12.0)];
    let loose = mono(&mut with(&loud), &input);
    let held = mono(&mut with(&[(BODY_GAIN, 12.0), (MATCH, 1.0)]), &input);
    let came = last(&input);

    assert!(
        last(&loose) > came * 2.0,
        "louder left alone: {}",
        last(&loose)
    );
    assert!(
        (last(&held) / came - 1.0).abs() < 0.1,
        "{} for {came}",
        last(&held)
    );
}

#[test]
fn the_output_trim_is_on_top_of_everything() {
    let input = line(&[(0.0, 440.0, 0.5), (0.3, 660.0, 0.5)], 0.4);
    let plain = mono(&mut with(&[]), &input);
    let down = mono(&mut with(&[(OUTPUT, -6.0)]), &input);

    assert!((rms(&down) / rms(&plain) - 0.501).abs() < 0.01);
}

#[test]
fn handing_over_makes_no_click() {
    let notes: Vec<(f32, f32, f32)> = (0..8)
        .map(|k| (k as f32 * 0.2, [440.0, 660.0][k % 2], 0.5))
        .collect();
    let input = line(&notes, 0.3);
    let out = mono(&mut with(&[(MATCH, 0.0)]), &input);
    // The plucks themselves start with a step; nothing else may be
    // steeper than a full 660 Hz wave at this level.
    let steepest_wave = std::f32::consts::TAU * 660.0 / RATE * 0.5;
    let step = out
        .windows(2)
        .enumerate()
        .filter(|(n, _)| notes.iter().all(|nt| n.abs_diff(s(nt.0)) > 2))
        .map(|(_, w)| (w[1] - w[0]).abs())
        .fold(0.0f32, f32::max);

    assert!(step < steepest_wave * 1.5, "{step}");
}

#[test]
fn a_sidechain_says_where_notes_start_when_asked_and_only_then() {
    // This track glides from one pitch to another with no jump in
    // level; the sidechain has two clicks.
    let tone: Vec<f32> = (0..s(1.0))
        .scan(0.0f32, |phase, n| {
            let hz = if n < s(0.3) { 440.0 } else { 660.0 };
            *phase += std::f32::consts::TAU * hz / RATE;
            Some(0.3 * phase.sin())
        })
        .collect();
    let clicks: Vec<f32> = (0..s(1.0))
        .map(
            |n| match n.abs_diff(s(0.1)) < 48 || n.abs_diff(s(0.5)) < 48 {
                true => 0.8,
                false => 0.0,
            },
        )
        .collect();
    let silence = vec![0.0; s(1.0)];
    let run = |key: &[f32], on: bool| {
        let input = [tone.clone(), tone.clone(), key.to_vec(), key.to_vec()];
        let settings = [(KEY, if on { 1.0 } else { 0.0 })];
        through(&mut with(&settings), &input).remove(0)
    };

    // Keyed, the note the sidechain starts at 0.5 s sings at the
    // pitch the track had after the one at 0.1 s: its body from 0.56
    // to 0.68 s is the track's from 0.16 to 0.28 s.
    let keyed = run(&clicks, true);
    let body = span(&keyed, 0.56, 0.68);
    assert!(near(body, 440.0) > near(body, 660.0) * 10.0);
    // Off, what is on the sidechain is not heard and starts nothing:
    // the track has no notes of its own, so it passes as it came.
    assert!(worst(&run(&clicks, false), &tone) < 1e-4);
    // On with nothing routed, nothing starts a note: not even this
    // track's own plucks.
    let plucks = line(&[(0.0, 440.0, 0.5), (0.3, 660.0, 0.5)], 0.7);
    let input = [plucks.clone(), plucks.clone(), silence.clone(), silence];
    let out = through(&mut with(&[(KEY, 1.0)]), &input).remove(0);
    assert!(worst(&out, &plucks) < 1e-4);
}

#[test]
fn soloing_the_sides_of_a_mono_sound_is_silence() {
    let input = line(&[(0.0, 440.0, 0.5), (0.3, 660.0, 0.5)], 0.4);
    let out = mono(&mut with(&[(LISTEN, 2.0)]), &input);

    assert!(rms(&out[s(0.1)..]) < 1e-4, "{}", rms(&out));
}

#[test]
fn a_moved_level_is_eased_not_jumped() {
    let input = vec![0.5; 4800];
    let mut runner = with(&[]);
    through(&mut runner, &[input.clone(), input.clone()]);
    runner.params[OUTPUT] = HandMeDown::PARAMS[OUTPUT].normalized(-24.0);
    let out = runner
        .render(&[input.clone(), input.clone()], input.len(), 64)
        .remove(0);
    let step = out
        .windows(2)
        .map(|w| (w[0] - w[1]).abs())
        .fold(0.0f32, f32::max);

    assert!(step < 0.01, "the biggest step between samples was {step}");
    assert!(out[4799] < 0.05, "and it got there: {}", out[4799]);
}

#[test]
fn nothing_from_before_a_reset_is_handed_down_after_it() {
    use kit::Runner;
    let rate = 48_000.0;
    // Hits a quarter of a second apart, each a burst that dies away.
    let hits = |level: f32| -> Vec<f32> {
        (0..rate as usize * 2)
            .map(|i| {
                let t = (i % 12_000) as f32 / rate;
                level * (-t * 30.0).exp() * (t * 2000.0).sin()
            })
            .collect()
    };
    let mut runner = Runner::<hand_me_down::HandMeDown>::new(rate, 256);
    let loud = hits(0.9);
    runner.render(&[loud.clone(), loud], rate as usize * 2, 256);
    // The memory is full of loud bodies. After a reset, quiet hits
    // must be handed only each other's.
    runner.reset();
    let quiet = hits(0.01);
    let out = runner.render(&[quiet.clone(), quiet], rate as usize * 2, 256);
    let peak = out[0].iter().fold(0.0f32, |m, v| m.max(v.abs()));
    assert!(peak < 0.05, "something loud came back: {peak}");
}
