//! Renders the demos someone listens to before trusting the tests:
//! `cargo run -p undertow --example undertow-demo -- OUT_DIR` writes one WAV a
//! setting worth hearing, each the dry sound then the same through
//! the plugin.
//!
//! The sound is made to show what Undertow is for: a bass line and a
//! voice in the middle, and in the sides a late copy of the voice and
//! a little hiss, as a room or a double would put there.

use kit::{Plugin, Runner};
use kit_test::{signals, wav};
use std::path::PathBuf;
use undertow::{DRIVE, FOLLOW, PUSH, SIDE_ON_MID, STEER, Undertow};

const RATE: f32 = 48_000.0;

// A bass line: four notes a bar, a soft square-ish tone.
fn bass(frames: usize) -> Vec<f32> {
    let notes = [55.0, 55.0, 65.4, 49.0];
    let beat = (RATE * 0.5) as usize;
    (0..frames)
        .map(|n| {
            let hz = notes[(n / beat) % notes.len()];
            let t = n as f32 / RATE;
            let fade = 1.0 - (n % beat) as f32 / beat as f32 * 0.6;
            let p = std::f32::consts::TAU * hz * t;
            (p.sin() + p.sin().powi(3) * 0.3) * 0.45 * fade
        })
        .collect()
}

fn source() -> Vec<Vec<f32>> {
    let voice = signals::sung(RATE);
    let frames = voice.len();
    let low = bass(frames);
    let late = (RATE * 0.023) as usize;
    let mut hiss = signals::noise(7);
    let mid: Vec<f32> = (0..frames).map(|n| low[n] + voice[n] * 0.5).collect();
    let side: Vec<f32> = (0..frames)
        .map(|n| {
            let echo = if n >= late { voice[n - late] } else { 0.0 };
            echo * 0.35 + hiss() * 0.01
        })
        .collect();
    vec![
        mid.iter().zip(&side).map(|(m, s)| m + s).collect(),
        mid.iter().zip(&side).map(|(m, s)| m - s).collect(),
    ]
}

fn main() -> std::io::Result<()> {
    let out = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "target/demos/undertow".into());
    let dry = source();
    let demos: [(&str, &[(usize, f64)]); 3] = [
        ("sides-ride-the-bass", &[(PUSH, 60.0), (DRIVE, 6.0)]),
        (
            "pushed-hard",
            &[(PUSH, 100.0), (DRIVE, 15.0), (FOLLOW, 120.0)],
        ),
        (
            "side-on-mid",
            &[(STEER, SIDE_ON_MID as f64), (PUSH, 80.0), (DRIVE, 9.0)],
        ),
    ];

    for (name, settings) in demos {
        let mut runner = Runner::<Undertow>::new(RATE, 512);
        for &(index, plain) in settings {
            runner.params[index] = Undertow::PARAMS[index].normalized(plain);
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
