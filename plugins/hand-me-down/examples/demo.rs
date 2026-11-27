//! Renders the demos someone listens to before trusting the tests:
//! `cargo run -p hand-me-down --example hand-me-down-demo -- OUT_DIR`
//! writes one WAV a setting worth hearing, each the dry sound then the
//! same through the plugin.
//!
//! Two sounds show what it is for: a plucked line, where each note's
//! pluck is now and its ring is the last note's, and a drum pattern,
//! where each drum's hit carries the body of the drum before it.

use hand_me_down::{BACK, HIT, HandMeDown, INTO, SIDE};
use kit::{Plugin, Runner};
use kit_test::{signals, wav};
use std::f32::consts::TAU;
use std::path::PathBuf;

const RATE: f32 = 48_000.0;

// A demo: its name, the dry sound, and the settings it is heard at.
type Demo = (&'static str, Vec<Vec<f32>>, &'static [(usize, f64)]);

fn at(seconds: f32) -> usize {
    (seconds * RATE) as usize
}

// A plucked string with a few harmonics, a little wider in the upper
// ones, so the sides have something of their own.
fn pluck(out: &mut [Vec<f32>], start: f32, hz: f32, amp: f32, length: f32) {
    let from = at(start);
    for n in 0..at(length) {
        let t = n as f32 / RATE;
        for (ch, buf) in out.iter_mut().enumerate() {
            let wide = if ch == 0 { 1.002 } else { 0.998 };
            let tone: f32 = (1..=5)
                .map(|k| {
                    let k = k as f32;
                    let detune = if k > 1.0 { wide } else { 1.0 };
                    (TAU * hz * k * detune * t).sin() * (-t * (3.0 + k * 2.5)).exp() / k
                })
                .sum();
            if let Some(x) = buf.get_mut(from + n) {
                *x += tone * amp;
            }
        }
    }
}

// Four bars of a line in A minor, eighth notes at 100 to the minute.
fn line() -> Vec<Vec<f32>> {
    let notes = [
        220.0, 261.6, 329.6, 440.0, 392.0, 329.6, 293.7, 261.6, 196.0, 246.9, 293.7, 392.0, 349.2,
        293.7, 246.9, 196.0,
    ];
    let step = 0.3;
    let length = notes.len() as f32 * step * 2.0 + 1.0;
    let mut out = vec![vec![0.0; at(length)]; 2];
    for k in 0..notes.len() * 2 {
        pluck(&mut out, k as f32 * step, notes[k % notes.len()], 0.4, 1.0);
    }
    out
}

// A kick, a snare and a closed hat, each with a body of its own.
fn drums() -> Vec<Vec<f32>> {
    let beat = 0.3;
    let length = beat * 32.0 + 1.0;
    let mut out = vec![0.0; at(length)];
    let mut hiss = signals::noise(11);
    let mut put = |start: f32, kind: usize| {
        let from = at(start);
        for n in 0..at(0.6) {
            let t = n as f32 / RATE;
            let x = match kind {
                // A kick: a falling sine.
                0 => {
                    (TAU * (45.0 * t + 90.0 * (1.0 - (-t * 30.0).exp()) / 30.0)).sin()
                        * (-t * 6.0).exp()
                        * 0.8
                }
                // A snare: a tone and noise.
                1 => ((TAU * 190.0 * t).sin() * 0.5 + hiss() * 0.6) * (-t * 14.0).exp() * 0.6,
                // A hat: noise, short.
                _ => hiss() * (-t * 60.0).exp() * 0.25,
            };
            if let Some(v) = out.get_mut(from + n) {
                *v += x;
            }
        }
    };
    let pattern = [0, 2, 1, 2, 0, 0, 1, 2];
    for k in 0..32 {
        put(k as f32 * beat, pattern[k % pattern.len()]);
    }
    vec![out.clone(), out]
}

fn main() -> std::io::Result<()> {
    let out = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "target/demos/hand-me-down".into());
    let demos: [Demo; 4] = [
        ("line-one-note-late", line(), &[]),
        ("line-two-notes-back", line(), &[(BACK, 1.0)]),
        ("line-late-in-the-sides", line(), &[(INTO, SIDE as f64)]),
        ("drums-bodies-passed-on", drums(), &[(HIT, 12.0)]),
    ];

    for (name, dry, settings) in demos {
        let mut runner = Runner::<HandMeDown>::new(RATE, 512);
        for &(index, plain) in settings {
            runner.params[index] = HandMeDown::PARAMS[index].normalized(plain);
        }
        let wet = runner.render(&dry, dry[0].len(), 512);
        let both: Vec<Vec<f32>> = wet
            .iter()
            .zip(&dry)
            .map(|(w, d)| d.iter().chain(w).copied().collect())
            .collect();
        let file = PathBuf::from(&out).join(format!("{name}.wav"));
        wav::write(&file, RATE as u32, &both)?;
        println!("{}", file.display());
    }
    Ok(())
}
