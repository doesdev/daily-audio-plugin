//! What Aftersound itself promises: written before the code that keeps
//! each promise.

use aftersound::{
    AFTER, Aftersound, BRIGHT, LISTEN, MISTUNE, OUTPUT, PROMPT, REFERENCE_HZ, RELEASE, STRIKE,
    foreseen,
};
use kit::{Event, Param, Plugin, Runner};

const RATE: f32 = 48_000.0;
const BLOCK: usize = 256;

// A runner with each named setting at a value in its own terms.
fn with(settings: &[(usize, f64)]) -> Runner<Aftersound> {
    let mut runner = Runner::<Aftersound>::new(RATE, BLOCK);
    for &(index, plain) in settings {
        let p: &Param = &Aftersound::PARAMS[index];
        runner.params[index] = p.normalized(plain);
    }
    runner
}

// Plays `notes` (seconds, MIDI message) for `seconds`; left and right.
fn play(settings: &[(usize, f64)], notes: &[(f32, [u8; 3])], seconds: f32) -> (Vec<f32>, Vec<f32>) {
    let mut runner = with(settings);
    let frames = (seconds * RATE) as usize;
    let (mut left, mut right) = (Vec::new(), Vec::new());
    let mut start = 0;
    while start < frames {
        let n = (frames - start).min(BLOCK);
        for &(at, data) in notes {
            let offset = (at * RATE) as usize;
            if (start..start + n).contains(&offset) {
                runner.push(Event {
                    offset: (offset - start) as u32,
                    data,
                });
            }
        }
        let out = runner.render(&[], n, n);
        left.extend_from_slice(&out[0]);
        right.extend_from_slice(&out[1]);
        start += n;
    }
    (left, right)
}

// One note held for the whole of `seconds`.
fn note(settings: &[(usize, f64)], key: u8, seconds: f32) -> (Vec<f32>, Vec<f32>) {
    play(settings, &[(0.0, [0x90, key, 100])], seconds)
}

fn mid_side(left: &[f32], right: &[f32]) -> (Vec<f32>, Vec<f32>) {
    left.iter()
        .zip(right)
        .map(|(l, r)| (0.5 * (l + r), 0.5 * (l - r)))
        .unzip()
}

fn rms(x: &[f32]) -> f32 {
    (x.iter().map(|s| s * s).sum::<f32>() / x.len().max(1) as f32).sqrt()
}

// The level of `x` between two times, in seconds.
fn during(x: &[f32], from: f32, to: f32) -> f32 {
    rms(&x[(from * RATE) as usize..(to * RATE) as usize])
}

// How strong `hz` is in `x` between two times: one bin of a DFT.
fn at_hz(x: &[f32], hz: f32, from: f32, to: f32) -> f32 {
    let piece = &x[(from * RATE) as usize..(to * RATE) as usize];
    let w = std::f32::consts::TAU * hz / RATE;
    let (re, im) = piece
        .iter()
        .enumerate()
        .fold((0.0, 0.0), |(re, im), (n, s)| {
            let a = w * n as f32;
            (re + s * a.cos(), im - s * a.sin())
        });
    2.0 * re.hypot(im) / piece.len() as f32
}

// The pitch of `x` between two times, from the lag that best repeats it.
fn pitch(x: &[f32], from: f32, to: f32, low: f32, high: f32) -> f32 {
    let piece = &x[(from * RATE) as usize..(to * RATE) as usize];
    let score = |lag: usize| -> f32 {
        let n = piece.len() - lag;
        (0..n).map(|i| piece[i] * piece[i + lag]).sum::<f32>() / n as f32
    };
    let lags = (RATE / high) as usize..=(RATE / low) as usize;
    let best = lags.max_by(|&a, &b| score(a).total_cmp(&score(b))).unwrap();
    let (a, b, c) = (score(best - 1), score(best), score(best + 1));
    let shift = 0.5 * (a - c) / (a - 2.0 * b + c);
    RATE / (best as f32 + shift)
}

// The first time, in steps of 50 ms, the side is louder than the middle.
fn takes_over(settings: &[(usize, f64)]) -> f32 {
    let (left, right) = note(settings, 57, 8.0);
    let (mid, side) = mid_side(&left, &right);
    (0..160)
        .map(|k| k as f32 * 0.05)
        .find(|&t| during(&side, t, t + 0.05) > during(&mid, t, t + 0.05))
        .unwrap_or(f32::INFINITY)
}

#[test]
fn a_note_starts_in_the_middle_and_blooms_into_the_sides() {
    // At its fundamental. The upper partials are mistuned by as many
    // times the mistuning as their number, so they spread sooner: the
    // shimmer of the strike is wide before the note's body is.
    let (left, right) = note(&[], 57, 5.0);
    let (mid, side) = mid_side(&left, &right);
    let hz = REFERENCE_HZ;
    let ratio = |t: f32| at_hz(&side, hz, t, t + 0.1) / at_hz(&mid, hz, t, t + 0.1);
    let (early, late) = (ratio(0.0), ratio(3.0));

    assert!(during(&mid, 0.0, 0.1) > 0.01, "it is heard at all");
    assert!(early < 0.2, "the side is {early} of the middle at first");
    assert!(late > 1.5, "and {late} of it three seconds on");
    // And over every partial, the sides grow against the middle.
    let wide = |t: f32| during(&side, t, t + 0.1) / during(&mid, t, t + 0.1);
    assert!(
        wide(3.0) > 2.0 * wide(0.0),
        "{} then {}",
        wide(0.0),
        wide(3.0)
    );
}

#[test]
fn strings_tuned_alike_and_struck_alike_never_leave_the_middle() {
    let (left, right) = note(&[(MISTUNE, 0.0), (STRIKE, 0.0)], 57, 2.0);

    assert!(rms(&left) > 0.003);
    assert_eq!(left, right, "the two strings are the same string");
}

#[test]
fn an_off_centre_strike_puts_the_sides_in_at_once() {
    let ratio = |strike: f64| {
        let (left, right) = note(&[(STRIKE, strike)], 57, 0.2);
        let (mid, side) = mid_side(&left, &right);
        let hz = REFERENCE_HZ;
        at_hz(&side, hz, 0.0, 0.1) / at_hz(&mid, hz, 0.0, 0.1)
    };

    let centred = ratio(0.0);
    for strike in [-60.0, 60.0] {
        let off = ratio(strike);
        assert!(
            off > 0.3 && off > centred * 3.0,
            "struck at {strike}, the side starts at {off}; evenly, {centred}"
        );
    }
}

#[test]
fn a_shorter_prompt_drops_the_middle_sooner() {
    let fall = |prompt: f64| {
        let (left, right) = note(&[(PROMPT, prompt), (MISTUNE, 0.0)], 57, 1.0);
        let (mid, _) = mid_side(&left, &right);
        during(&mid, 0.6, 0.8) / during(&mid, 0.0, 0.1)
    };
    let (short, long) = (fall(0.15), fall(2.0));

    assert!(short < long / 10.0, "{short} short, {long} long");
}

#[test]
fn a_longer_aftersound_keeps_the_sides_ringing() {
    let ring = |after: f64| {
        let (left, right) = note(&[(AFTER, after)], 57, 4.0);
        let (_, side) = mid_side(&left, &right);
        during(&side, 3.0, 4.0)
    };
    let (short, long) = (ring(1.5), ring(25.0));

    assert!(long > short * 5.0, "{short} short, {long} long");
}

#[test]
fn more_mistune_hands_over_to_the_sides_sooner() {
    let (slight, wide) = (takes_over(&[(MISTUNE, 0.3)]), takes_over(&[(MISTUNE, 2.5)]));

    assert!(wide < slight * 0.6, "{wide} s wide, {slight} s slight");
}

#[test]
fn it_plays_in_tune_up_and_down_the_keys() {
    for (key, hz) in [(45u8, 110.0f32), (69, 440.0), (93, 1760.0)] {
        let settings = [(MISTUNE, 0.0), (BRIGHT, 0.0), (PROMPT, 4.0)];
        let (left, _) = note(&settings, key, 1.0);
        let heard = pitch(&left, 0.3, 0.8, hz * 0.9, hz * 1.1);
        let cents = 1200.0 * (heard / hz).log2();

        assert!(
            cents.abs() < 3.0,
            "key {key}: {heard} Hz, {cents} cents off"
        );
    }
}

#[test]
fn the_two_strings_are_tuned_apart_by_the_mistune() {
    // The sides alone are what the strings do against each other: their
    // two pitches, each half the mistuning from the note.
    let settings = [
        (MISTUNE, 4.0),
        (BRIGHT, 0.0),
        (PROMPT, 4.0),
        (STRIKE, 100.0),
    ];
    let (left, right) = note(&settings, 57, 3.0);
    let (l, r) = (
        pitch(&left, 1.0, 3.0, 200.0, 240.0),
        pitch(&right, 1.0, 3.0, 200.0, 240.0),
    );

    assert!(r > l + 1.0, "left at {l} Hz, right at {r} Hz");
}

#[test]
fn the_sound_follows_the_curves_the_picture_draws() {
    // At the fundamental, where `foreseen` works, the side against the
    // middle as it is heard and as it is drawn.
    let (prompt, after, mistune) = (0.6f32, 7.0f32, 0.7f32);
    let settings = [
        (PROMPT, prompt as f64),
        (AFTER, after as f64),
        (MISTUNE, mistune as f64),
        (BRIGHT, 0.0),
    ];
    let (left, right) = note(&settings, 57, 4.0);
    let (mid, side) = mid_side(&left, &right);

    for t in [0.5f32, 1.0, 2.0, 3.0] {
        let hz = REFERENCE_HZ;
        let heard = at_hz(&side, hz, t, t + 0.2) / at_hz(&mid, hz, t, t + 0.2);
        let (m, s) = foreseen(prompt, after, mistune, 0.0, t + 0.1);
        let drawn = s / m;
        let off = 20.0 * (heard / drawn).log10();

        assert!(off.abs() < 3.0, "at {t} s: heard {heard}, drawn {drawn}");
    }
}

#[test]
fn foreseen_starts_whole_in_the_middle_and_hands_over() {
    let cases = [
        // prompt, after, mistune, strike, seconds, mid, side
        (0.6, 9.0, 0.7, 0.0, 0.0, 1.0, 0.0),
        (0.6, 9.0, 0.0, 0.5, 0.0, 1.0, 0.5),
        (0.6, 9.0, 0.0, 0.0, 0.6, 0.001, 0.0),
        (9.0, 9.0, 0.0, 0.0, 9.0, 0.001, 0.0),
        (0.6, 9.0, 0.0, 1.0, 9.0, 0.0, 0.001),
    ];

    for (prompt, after, mistune, strike, t, mid, side) in cases {
        let (m, s) = foreseen(prompt, after, mistune, strike, t);
        let close = |got: f32, want: f32| (got - want).abs() <= want * 0.05 + 1e-5;
        assert!(close(m, mid) && close(s, side), "{t} s: {m} {s}");
    }
    // Mistuned, the middle falls and the side rises past it.
    let (m, s) = foreseen(0.6, 9.0, 0.7, 0.0, 2.0);
    assert!(s > m * 3.0 && s > 0.01, "{m} {s}");
}

#[test]
fn letting_go_damps_the_note() {
    let notes = [(0.0, [0x90, 57, 100]), (1.0, [0x80, 57, 0])];
    let (left, right) = play(&[(RELEASE, 0.2)], &notes, 1.6);
    let (before, after) = (during(&left, 0.9, 1.0), during(&left, 1.4, 1.5));

    assert!(after < before * 1e-2, "{before} before, {after} after");
    assert!(during(&right, 1.4, 1.5) < before * 1e-2);
}

#[test]
fn harder_is_louder() {
    let level = |velocity: u8| {
        let (left, _) = play(&[], &[(0.0, [0x90, 57, velocity])], 0.5);
        during(&left, 0.0, 0.5)
    };

    assert!(level(120) > level(30) * 3.0);
}

#[test]
fn each_half_can_be_heard_alone() {
    let cases = [(1.0, 1.0f32), (2.0, -1.0)];

    for (listen, sign) in cases {
        let (left, right) = note(&[(LISTEN, listen), (STRIKE, 50.0)], 57, 1.0);
        let worst = left
            .iter()
            .zip(&right)
            .skip(RATE as usize / 2)
            .map(|(l, r)| (l - sign * r).abs())
            .fold(0.0f32, f32::max);

        assert!(rms(&left[24_000..]) > 1e-3, "listen {listen} is heard");
        assert!(worst < 1e-4, "listen {listen}: off by {worst}");
    }
}

#[test]
fn the_output_trim_sets_the_level() {
    let level = |db: f64| {
        let (left, _) = note(&[(OUTPUT, db)], 57, 0.5);
        during(&left, 0.1, 0.5)
    };
    let ratio = level(-12.0) / level(0.0);

    assert!((ratio - 0.251).abs() < 0.01, "{ratio}");
}

#[test]
fn foreseen_gives_the_numbers_the_page_is_held_to() {
    // The same table is in ui/test.js, against ui/foreseen.js: change
    // one and change the other.
    let cases = [
        // prompt, after, mistune, strike, seconds, mid, side
        (0.6, 7.0, 0.5, 0.0, 0.25, 0.043_568_2, 0.108_273),
        (0.6, 7.0, 0.5, 0.0, 1.0, 0.006_987_57, 0.046_954_5),
        (0.6, 7.0, 0.5, 0.0, 3.0, 0.000_602_003, 0.004_037_94),
        (0.3, 20.0, 1.5, 0.4, 1.0, 0.025_356_2, 0.122_723),
        (1.2, 4.0, 2.5, -0.2, 0.8, 0.051_834_1, 0.014_307_8),
    ];

    for (prompt, after, mistune, strike, t, mid, side) in cases {
        let (m, s) = foreseen(prompt, after, mistune, strike, t);
        let close = |got: f32, want: f32| (got / want - 1.0).abs() < 0.01;
        assert!(close(m, mid) && close(s, side), "{t} s: {m} {s}");
    }
}

#[test]
fn strike_leans_the_hammer_toward_the_side_it_names() {
    let first = |strike: f64| {
        let (left, right) = note(&[(STRIKE, strike), (MISTUNE, 0.0)], 57, 0.05);
        (during(&left, 0.0, 0.01), during(&right, 0.0, 0.01))
    };
    let ((l_left, r_left), (l_right, r_right)) = (first(-80.0), first(80.0));

    assert!(l_left > r_left * 2.0, "toward the left: {l_left} {r_left}");
    assert!(
        r_right > l_right * 2.0,
        "toward the right: {l_right} {r_right}"
    );
}

#[test]
fn the_sustain_pedal_holds_a_let_go_note_until_it_is_lifted() {
    let pedal = |down: bool| [0xB0, 64, if down { 127 } else { 0 }];
    let held = [
        (0.0, pedal(true)),
        (0.0, [0x90, 57, 100]),
        (0.5, [0x80, 57, 0]),
        (1.5, pedal(false)),
    ];
    let (left, _) = play(&[(RELEASE, 0.1)], &held, 2.2);
    let (before, pedalled, lifted) = (
        during(&left, 0.4, 0.5),
        during(&left, 1.3, 1.4),
        during(&left, 2.0, 2.1),
    );

    assert!(
        pedalled > before * 0.1,
        "{before} then {pedalled} on the pedal"
    );
    assert!(lifted < pedalled * 1e-3, "{pedalled} then {lifted} lifted");
}
