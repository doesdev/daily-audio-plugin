//! What Ironside itself promises: written before the code that keeps
//! each promise.

use ironside::{
    CEILING, CORNER, FILL, FULL, IRON_DRIVE, IRON_MATERIAL, Ironside, KEY, LISTEN, MATCH, MIX,
    OUTPUT, RECOVER, corner_of,
};
use kit::{Param, Plugin, Runner};
use kit_test::signals;

const RATE: f32 = 48_000.0;
const SECOND: usize = RATE as usize;

// A runner with each named setting at a value in its own terms.
fn with(settings: &[(usize, f64)]) -> Runner<Ironside> {
    let mut runner = Runner::<Ironside>::new(RATE, 256);
    for &(index, plain) in settings {
        let p: &Param = &Ironside::PARAMS[index];
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
// been eased to and the filters have settled, lined up with what
// went in by as late as the plugin says it is.
fn settled(runner: &mut Runner<Ironside>, input: &[Vec<f32>]) -> (Vec<f32>, Vec<f32>) {
    let lead: Vec<Vec<f32>> = input
        .iter()
        .map(|ch| (0..SECOND / 2).map(|i| ch[i % ch.len()]).collect())
        .collect();
    runner.render(&lead, lead[0].len(), 256);
    let late = runner.latency() as usize;
    let longer: Vec<Vec<f32>> = input
        .iter()
        .map(|ch| {
            ch.iter()
                .chain(ch.iter().cycle().take(late))
                .copied()
                .collect()
        })
        .collect();
    let out = runner.render(&longer, longer[0].len(), 256);
    let (left, right) = (&out[0][late..], &out[1][late..]);
    let mid = left.iter().zip(right).map(|(l, r)| (l + r) / 2.0);
    let side = left.iter().zip(right).map(|(l, r)| (l - r) / 2.0);
    (mid.collect(), side.collect())
}

fn worst(a: &[f32], b: &[f32]) -> f32 {
    a.iter()
        .zip(b)
        .map(|(x, y)| (x - y).abs())
        .fold(0.0, f32::max)
}

fn rms(x: &[f32]) -> f32 {
    (x.iter().map(|s| s * s).sum::<f32>() / x.len() as f32).sqrt()
}

// How much of `x` is at `hz`, as an amplitude.
fn at(x: &[f32], hz: f32) -> f32 {
    let w = std::f32::consts::TAU * hz / RATE;
    let (re, im) = x.iter().enumerate().fold((0.0, 0.0), |(re, im), (n, s)| {
        let p = w * n as f32;
        (re + s * p.cos(), im + s * p.sin())
    });
    (re * re + im * im).sqrt() * 2.0 / x.len() as f32
}

fn sine(hz: f32, gain: f32) -> Vec<f32> {
    signals::sine(RATE, 1.0, hz, gain)
}

fn silence() -> Vec<f32> {
    vec![0.0; SECOND]
}

fn db(ratio: f32) -> f32 {
    20.0 * ratio.max(1e-9).log10()
}

// How much of a side tone at `side_hz` gets through beside a middle
// of `mid`, in dB against what gets through with a silent middle.
fn kept(settings: &[(usize, f64)], mid: &[f32], side_hz: f32) -> f32 {
    let tone = sine(side_hz, 0.1);
    let (_, alone) = settled(&mut with(settings), &stereo(&silence(), &tone));
    let (_, beside) = settled(&mut with(settings), &stereo(mid, &tone));
    db(at(&beside, side_hz) / at(&alone, side_hz))
}

#[test]
fn the_corner_climbs_as_the_core_closes_and_stops_at_the_ceiling() {
    // The same numbers are in ui/test.js: the picture's corner and the
    // sound's must be one.
    let cases = [
        (30.0, 1200.0, 1.0, 30.0),
        (30.0, 1200.0, 0.5, 60.0),
        (30.0, 1200.0, 0.1, 300.0),
        (30.0, 1200.0, 0.01, 1200.0),
        (30.0, 1200.0, 0.0, 1200.0),
        (100.0, 5000.0, 0.25, 400.0),
        // A ceiling under the corner leaves the corner where it is.
        (300.0, 100.0, 0.5, 300.0),
    ];
    for (rest, ceiling, open, want) in cases {
        let got = corner_of(rest, ceiling, open);
        assert!((got - want).abs() < 1e-3, "{rest} {ceiling} {open}: {got}");
    }
}

#[test]
fn a_mono_sound_comes_out_exactly_as_it_went_in() {
    let voice = signals::sung(RATE);
    let input = stereo(&voice, &vec![0.0; voice.len()]);
    let (mid, side) = settled(&mut with(&[(FILL, 100.0)]), &input);
    assert!(worst(&mid, &voice) < 1e-5, "{}", worst(&mid, &voice));
    assert!(side.iter().all(|s| s.abs() < 1e-6));
}

#[test]
fn it_says_how_late_it_is_and_is_later_when_rendering() {
    let voice = signals::sung(RATE);
    let side: Vec<f32> = (0..voice.len())
        .map(|i| 0.2 * (i as f32 * 0.11).sin())
        .collect();
    let input = stereo(&voice, &side);
    let mut lates = Vec::new();
    for rendering in [false, true] {
        let mut runner = with(&[(MIX, 0.0), (IRON_DRIVE, 100.0)]);
        runner.set_rendering(rendering);
        let late = runner.latency();
        let (mid, out_side) = settled(&mut runner, &input);
        assert!(late > 0, "rendering {rendering}");
        assert!(worst(&mid, &voice) < 1e-5, "rendering {rendering}");
        assert!(worst(&out_side, &side) < 1e-5, "rendering {rendering}");
        lates.push(late);
    }
    assert!(lates[1] > lates[0], "{lates:?}");
}

#[test]
fn with_the_core_empty_the_sides_come_through_above_the_corner() {
    let tone = sine(1000.0, 0.3);
    let (mid, side) = settled(&mut with(&[]), &stereo(&silence(), &tone));
    assert!(mid.iter().all(|m| m.abs() < 1e-6));
    let lost = db(at(&side, 1000.0) / 0.3);
    assert!(lost.abs() < 0.05, "{lost} dB");
    // And below the corner they go: a first-order slope.
    let deep = sine(10.0, 0.3);
    let (_, side) = settled(&mut with(&[(CORNER, 60.0)]), &stereo(&silence(), &deep));
    assert!(db(at(&side, 10.0) / 0.3) < -12.0);
}

#[test]
fn the_middles_bass_takes_the_sides_lows() {
    let bass = sine(60.0, 0.5);
    let lost = kept(&[], &bass, 150.0);
    assert!(lost < -6.0, "{lost} dB");
}

#[test]
fn it_moves_a_corner_and_does_not_duck_the_sides() {
    let bass = sine(60.0, 0.5);
    let low = kept(&[], &bass, 150.0);
    let high = kept(&[], &bass, 5000.0);
    assert!(low < -6.0, "{low} dB");
    assert!(high > -1.5, "{high} dB");
}

#[test]
fn a_low_note_fills_the_core_sooner_than_a_high_one_at_the_same_level() {
    let low = kept(&[], &sine(50.0, 0.3), 200.0);
    let high = kept(&[], &sine(400.0, 0.3), 200.0);
    assert!(
        low < high - 6.0,
        "{low} dB under 50 Hz, {high} dB under 400 Hz"
    );
    assert!(high > -1.0, "{high} dB");
}

#[test]
fn with_no_fill_the_bass_does_nothing_to_the_sides() {
    let lost = kept(&[(FILL, 0.0)], &sine(60.0, 0.8), 150.0);
    assert!(lost.abs() < 0.01, "{lost} dB");
}

#[test]
fn at_no_recover_the_sides_open_and_shut_twice_in_each_wave_of_the_bass() {
    // The crests of the bass close the core either way up, so the
    // side tone is shaped at twice the bass and grows sidebands there.
    let grain = |recover: f64| {
        let settings = [(RECOVER, recover), (FILL, 40.0)];
        let tone = sine(1000.0, 0.1);
        let (_, side) = settled(&mut with(&settings), &stereo(&sine(60.0, 0.4), &tone));
        (at(&side, 880.0) + at(&side, 1120.0)) / at(&side, 1000.0)
    };
    let quick = grain(0.0);
    let slow = grain(300.0);
    assert!(quick > 0.03, "{quick}");
    assert!(slow < quick * 0.3, "{slow} against {quick}");
}

#[test]
fn the_ceiling_is_as_high_as_the_corner_can_be_pushed() {
    let bass = sine(40.0, 0.9);
    let under = kept(&[(CEILING, 0.2), (FILL, 100.0)], &bass, 1000.0);
    let over = kept(&[(CEILING, 5.0), (FILL, 100.0)], &bass, 1000.0);
    assert!(under > -1.5, "{under} dB");
    assert!(over < under - 3.0, "{over} dB");
}

#[test]
fn steel_fills_on_a_quieter_bass_than_nickel() {
    // Nickel holds straight until its knee; steel bends from the start.
    let full = |settings: &[(usize, f64)]| {
        let mut runner = with(settings);
        settled(&mut runner, &stereo(&sine(60.0, 0.0625), &sine(300.0, 0.1)));
        runner.meters()[FULL]
    };
    let nickel = full(&[]);
    let steel = full(&[(IRON_MATERIAL, 100.0)]);
    assert!(nickel < 0.12, "{nickel}");
    assert!(steel > nickel * 2.0, "{steel} against {nickel}");
}

#[test]
fn the_transformer_window_bends_the_middle_and_at_rest_changes_nothing() {
    let bass = sine(60.0, 0.5);
    let side = vec![0.0; bass.len()];
    let third = |settings: &[(usize, f64)]| {
        let (mid, _) = settled(&mut with(settings), &stereo(&bass, &side));
        at(&mid, 180.0)
    };
    assert!(third(&[]) < 1e-5, "{}", third(&[]));
    assert!(third(&[(IRON_DRIVE, 100.0)]) > 0.005);
}

#[test]
fn with_no_mix_the_sound_is_the_input() {
    let bass = sine(60.0, 0.6);
    let tone = sine(150.0, 0.2);
    let input = stereo(&bass, &tone);
    let (mid, side) = settled(&mut with(&[(MIX, 0.0), (IRON_DRIVE, 100.0)]), &input);
    assert!(worst(&mid, &bass) < 1e-5);
    assert!(worst(&side, &tone) < 1e-5);
}

#[test]
fn the_full_meter_shows_how_far_the_middle_has_filled_the_core() {
    let full = |mid: &[f32]| {
        let mut runner = with(&[]);
        let tone = sine(300.0, 0.1);
        settled(&mut runner, &stereo(mid, &tone));
        runner.meters()[FULL]
    };
    assert!(full(&silence()) < 0.01);
    let some = full(&sine(60.0, 0.15));
    let more = full(&sine(60.0, 0.6));
    assert!(some > 0.05 && some < 0.9, "{some}");
    assert!(more > some && more > 0.9, "{more}");
}

#[test]
fn a_moved_setting_is_eased_not_jumped() {
    let input = stereo(&vec![0.3; 4800], &vec![0.1; 4800]);
    let mut runner = with(&[(OUTPUT, 0.0)]);
    settled(&mut runner, &input);
    runner.params[OUTPUT] = Ironside::PARAMS[OUTPUT].normalized(-24.0);
    let out = runner.render(&input, 4800, 64).remove(0);
    let step = out
        .windows(2)
        .map(|w| (w[0] - w[1]).abs())
        .fold(0.0f32, f32::max);
    assert!(step < 0.01, "{step}");
    assert!(out[4799].abs() < 0.05, "{}", out[4799]);
}

#[test]
fn matched_it_goes_out_as_loud_as_it_came_in_however_full_the_core() {
    let bass = signals::sine(RATE, 3.0, 55.0, 0.5);
    let tone = signals::sine(RATE, 3.0, 110.0, 0.4);
    let input = stereo(&bass, &tone);
    let last = |x: &[f32]| rms(&x[x.len() - SECOND..]);
    let came = (last(&input[0]).powi(2) + last(&input[1]).powi(2)).sqrt();
    let went = |settings: &[(usize, f64)]| {
        let (mid, side) = settled(&mut with(settings), &input);
        let (l, r): (Vec<f32>, Vec<f32>) =
            mid.iter().zip(&side).map(|(m, s)| (m + s, m - s)).unzip();
        (last(&l).powi(2) + last(&r).powi(2)).sqrt()
    };
    let loose = went(&[(FILL, 100.0), (IRON_DRIVE, 100.0)]);
    let held = went(&[(FILL, 100.0), (IRON_DRIVE, 100.0), (MATCH, 1.0)]);
    assert!(loose < came * 0.95, "{loose} for {came}");
    assert!((held / came - 1.0).abs() < 0.03, "{held} for {came}");
}

#[test]
fn soloed_only_that_half_is_heard() {
    let bass = sine(60.0, 0.4);
    let tone = sine(1000.0, 0.2);
    let input = stereo(&bass, &tone);
    let (mid, side) = settled(&mut with(&[(LISTEN, 2.0)]), &input);
    assert!(rms(&mid) < 0.01, "{}", rms(&mid));
    assert!(rms(&side) > 0.1);
}

#[test]
fn a_sidechain_fills_the_core_when_asked_and_only_then() {
    let bass = sine(60.0, 0.5);
    let tone = sine(150.0, 0.1);
    let own = stereo(&silence(), &tone);
    let keyed = [own.clone(), vec![bass.clone(), bass.clone()]].concat();
    let side_of = |input: &[Vec<f32>], settings: &[(usize, f64)]| {
        let (_, side) = settled(&mut with(settings), input);
        at(&side, 150.0)
    };
    let open = side_of(&own, &[]);
    let on = [(KEY, 1.0)];

    // Another track's bass fills this one's core.
    assert!(side_of(&keyed, &on) < open * 0.5);
    // Switched off, the sidechain is neither heard nor felt.
    assert!((side_of(&keyed, &[]) / open - 1.0).abs() < 1e-3);
    // Switched on with nothing routed, nothing fills it: its own bass
    // no longer does.
    let bassy = stereo(&bass, &tone);
    assert!((side_of(&bassy, &on) / open - 1.0).abs() < 1e-3);
    assert!(side_of(&bassy, &[]) < open * 0.5);
}

#[test]
fn the_core_is_as_open_at_each_flux_as_the_page_draws_it() {
    // The same numbers are in ui/test.js.
    let cases = [
        (0.0, 4.0, 0.0, 1.0),
        (0.1, 8.0, 0.0, 0.651_074),
        (0.25, 8.0, 0.0, 0.028_969),
        (0.25, 8.0, 1.0, 0.070_651),
        (0.1, 4.0, 1.0, 0.855_639),
        (0.5, 0.0, 1.0, 1.0),
    ];
    for (x, push, iron, want) in cases {
        let got = kit_dsp::iron_slope(x, push, iron);
        assert!((got - want).abs() < 1e-5, "{x} {push} {iron}: {got}");
    }
}

#[test]
fn the_sides_path_is_a_first_order_high_pass_as_the_page_draws_it() {
    // The same numbers are in ui/test.js: with the middle silent the
    // corner stands at Corner, and Mix lays the dry sides back in.
    let cases = [
        (100.0, 100.0, 100.0, -3.0103),
        (50.0, 100.0, 100.0, -6.9897),
        (100.0, 100.0, 50.0, -2.0412),
        (100.0, 100.0, 0.0, 0.0),
    ];
    for (hz, corner, mix, want) in cases {
        let tone = sine(hz, 0.2);
        let settings = [(CORNER, corner), (MIX, mix)];
        let (_, side) = settled(&mut with(&settings), &stereo(&silence(), &tone));
        let got = db(at(&side, hz) / 0.2);
        assert!((got - want).abs() < 0.05, "{hz} Hz at {mix}%: {got} dB");
    }
}

#[test]
fn it_is_as_late_as_the_manual_says() {
    // docs/manual.md gives these two numbers.
    let mut runner = with(&[]);
    assert_eq!(runner.latency(), 39);
    runner.set_rendering(true);
    runner.render(&[vec![0.0; 64], vec![0.0; 64]], 64, 64);
    assert_eq!(runner.latency(), 65);
}
