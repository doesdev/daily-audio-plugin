//! What Template itself promises: written before the code that keeps
//! each promise.

use kit::{Param, Plugin, Runner};
use kit_test::signals;
use template::{DRIVE, FOLD, HARD, MIX, OUTPUT, SHAPE, SOFT, TONE, Template, shaped};

const RATE: f32 = 48_000.0;

// A runner with each named setting at a value in its own terms.
fn with(settings: &[(usize, f64)]) -> Runner<Template> {
    let mut runner = Runner::<Template>::new(RATE, 256);
    for &(index, plain) in settings {
        let p: &Param = &Template::PARAMS[index];
        runner.params[index] = p.normalized(plain);
    }
    runner
}

// The left channel for `input`, once the settings have been eased to.
fn settled(runner: &mut Runner<Template>, input: &[f32]) -> Vec<f32> {
    let lead = vec![vec![0.0; RATE as usize / 2]];
    runner.render(&lead, lead[0].len(), 256);
    runner.render(&[input.to_vec()], input.len(), 256).remove(0)
}

fn rms(x: &[f32]) -> f32 {
    (x.iter().map(|s| s * s).sum::<f32>() / x.len() as f32).sqrt()
}

#[test]
fn the_shapers_bend_what_goes_past_the_limit() {
    let cases = [
        (SOFT, 0.2, 0.2f32.tanh()),
        (SOFT, 9.0, 1.0),
        (HARD, 0.2, 0.2),
        (HARD, 3.0, 1.0),
        (HARD, -3.0, -1.0),
        (FOLD, 0.2, 0.2),
        (FOLD, 1.5, 0.5),
        (FOLD, 3.0, -1.0),
        (FOLD, -1.5, -0.5),
    ];

    for (shape, x, want) in cases {
        let got = shaped(shape, x);
        assert!((got - want).abs() < 1e-4, "shape {shape} at {x}: {got}");
    }
}

#[test]
fn dry_is_the_input_at_the_output_level() {
    let input = signals::sung(RATE);
    let mut runner = with(&[(MIX, 0.0), (OUTPUT, 0.0), (DRIVE, 36.0)]);
    let out = settled(&mut runner, &input);
    let worst = out
        .iter()
        .zip(&input)
        .map(|(o, i)| (o - i).abs())
        .fold(0.0f32, f32::max);

    assert!(worst < 1e-4, "off the dry signal by up to {worst}");
}

#[test]
fn more_drive_is_louder_and_never_past_the_rails() {
    let input = signals::sine(RATE, 0.5, 220.0, 0.25);
    let level = |drive: f64, shape: usize| {
        let mut runner = with(&[
            (DRIVE, drive),
            (SHAPE, shape as f64),
            (OUTPUT, 0.0),
            (TONE, 16_000.0),
        ]);
        let out = settled(&mut runner, &input);
        (rms(&out), out.iter().fold(0.0f32, |m, s| m.max(s.abs())))
    };

    for shape in [SOFT, HARD] {
        let (quiet, _) = level(0.0, shape);
        let (loud, peak) = level(24.0, shape);
        assert!(loud > quiet * 2.0, "shape {shape}: {quiet} then {loud}");
        // The filter rings a little past the shaper's own limit of 1.
        assert!(peak < 1.3, "shape {shape}: peak {peak}");
    }
}

#[test]
fn tone_takes_the_top_off() {
    let input = signals::white(RATE, 0.5);
    let level = |tone: f64| {
        let mut runner = with(&[(TONE, tone), (DRIVE, 0.0), (SHAPE, HARD as f64)]);
        rms(&settled(&mut runner, &input))
    };
    let (open, closed) = (level(16_000.0), level(200.0));

    assert!(closed < open * 0.25, "{open} open, {closed} closed");
}

#[test]
fn a_moved_setting_is_eased_not_jumped() {
    let input = vec![0.5; 4800];
    let mut runner = with(&[(MIX, 0.0), (OUTPUT, 0.0)]);
    settled(&mut runner, &input);
    runner.params[OUTPUT] = Template::PARAMS[OUTPUT].normalized(-24.0);
    let out = runner
        .render(std::slice::from_ref(&input), input.len(), 64)
        .remove(0);
    let step = out
        .windows(2)
        .map(|w| (w[0] - w[1]).abs())
        .fold(0.0f32, f32::max);

    assert!(step < 0.01, "the biggest step between samples was {step}");
    assert!(out[4799] < 0.05, "and it got there: {}", out[4799]);
}

#[test]
fn matched_it_goes_out_as_loud_as_it_came_in_whatever_the_drive() {
    let tone: Vec<f32> = (0..RATE as usize * 3)
        .map(|i| 0.25 * (i as f32 * 0.03).sin())
        .collect();
    let last = |x: &[f32]| rms(&x[x.len() - RATE as usize..]);
    let came = last(&tone);

    for drive in [0.0, 18.0, 36.0] {
        let settings = [(DRIVE, drive), (OUTPUT, 0.0)];
        let loose = last(&settled(&mut with(&settings), &tone));
        let matched = [settings.as_slice(), &[(template::MATCH, 1.0)]].concat();
        let held = last(&settled(&mut with(&matched), &tone));

        assert!(
            (held / came - 1.0).abs() < 0.03,
            "{drive} dB: {held} for {came}"
        );
        // Left alone, driving it is simply louder: that is what
        // matching is for.
        assert!(drive == 0.0 || loose > came * 1.5, "{drive} dB: {loose}");
    }
}

#[test]
fn the_output_trim_is_still_the_persons_own_on_top_of_matching() {
    let tone: Vec<f32> = (0..RATE as usize * 3)
        .map(|i| 0.25 * (i as f32 * 0.03).sin())
        .collect();
    let last = |x: &[f32]| rms(&x[x.len() - RATE as usize..]);
    let settings = [(DRIVE, 24.0), (OUTPUT, -6.0), (template::MATCH, 1.0)];
    let held = last(&settled(&mut with(&settings), &tone));

    assert!((held / last(&tone) - 0.501).abs() < 0.02, "{held}");
}
