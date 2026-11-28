//! Renders the demos someone listens to before trusting the tests:
//! `cargo run -p ironside --example ironside-demo -- OUT_DIR` writes one
//! WAV a setting worth hearing, each the dry sound then the same
//! through the plugin.
//!
//! The sound is made to show what Ironside is for: a bass line and a
//! voice in the middle, and in the sides a wide chord with plenty of
//! low middle in it, a late copy of the voice and a little hiss.

use ironside::{CEILING, FILL, IRON_DRIVE, IRON_MATERIAL, Ironside, RECOVER};
use kit::{Plugin, Runner};
use kit_test::{signals, wav};
use std::path::PathBuf;

const RATE: f32 = 48_000.0;

// A bass line: four notes a bar, each dying away.
fn bass(frames: usize) -> Vec<f32> {
    let notes = [55.0, 55.0, 65.4, 49.0];
    let beat = (RATE * 0.5) as usize;
    (0..frames)
        .map(|n| {
            let hz = notes[(n / beat) % notes.len()];
            let t = n as f32 / RATE;
            let fade = 1.0 - (n % beat) as f32 / beat as f32 * 0.6;
            let p = std::f32::consts::TAU * hz * t;
            (p.sin() + p.sin().powi(3) * 0.3) * 0.4 * fade
        })
        .collect()
}

// A chord held wide: each note a little apart left and right, so it
// lives in the sides, from the low middle up.
fn chord(frames: usize) -> Vec<f32> {
    let notes = [110.0, 164.8, 220.0, 277.2, 440.0];
    (0..frames)
        .map(|n| {
            let t = n as f32 / RATE;
            notes
                .iter()
                .enumerate()
                .map(|(k, hz)| {
                    let drift = 1.0 + 0.002 * (k as f32 + 1.0);
                    (std::f32::consts::TAU * hz * drift * t).sin() * 0.06
                })
                .sum()
        })
        .collect()
}

fn source() -> Vec<Vec<f32>> {
    let voice = signals::sung(RATE);
    let frames = voice.len();
    let (low, wide) = (bass(frames), chord(frames));
    let late = (RATE * 0.023) as usize;
    let mut hiss = signals::noise(7);
    let mid: Vec<f32> = (0..frames).map(|n| low[n] + voice[n] * 0.5).collect();
    let side: Vec<f32> = (0..frames)
        .map(|n| {
            let echo = if n >= late { voice[n - late] } else { 0.0 };
            wide[n] + echo * 0.25 + hiss() * 0.01
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
        .unwrap_or_else(|| "target/demos/ironside".into());
    let dry = source();
    let demos: [(&str, &[(usize, f64)]); 3] = [
        ("sides-give-way-to-the-bass", &[(FILL, 50.0)]),
        (
            "grain-at-every-swing",
            &[(FILL, 80.0), (RECOVER, 0.0), (CEILING, 3.0)],
        ),
        (
            "steel-and-slow",
            &[
                (FILL, 40.0),
                (RECOVER, 180.0),
                (IRON_MATERIAL, 100.0),
                (IRON_DRIVE, 40.0),
            ],
        ),
    ];

    for (name, settings) in demos {
        let mut runner = Runner::<Ironside>::new(RATE, 512);
        for &(index, plain) in settings {
            runner.params[index] = Ironside::PARAMS[index].normalized(plain);
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
