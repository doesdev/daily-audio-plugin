//! Renders the demos someone listens to before trusting the tests:
//! `cargo run -p template --example template-demo -- OUT_DIR` writes one WAV a
//! setting worth hearing, each the dry signal then the same through
//! the plugin.

use kit::{Plugin, Runner};
use kit_test::{signals, wav};
use std::path::PathBuf;
use template::{DRIVE, FOLD, HARD, SHAPE, SOFT, TONE, Template};

const RATE: f32 = 48_000.0;

fn main() -> std::io::Result<()> {
    let out = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "target/demos/template".into());
    let dry = signals::sung(RATE);
    let demos: [(&str, &[(usize, f64)]); 3] = [
        (
            "soft-warm",
            &[(SHAPE, SOFT as f64), (DRIVE, 18.0), (TONE, 3000.0)],
        ),
        (
            "hard-bright",
            &[(SHAPE, HARD as f64), (DRIVE, 24.0), (TONE, 12_000.0)],
        ),
        (
            "fold",
            &[(SHAPE, FOLD as f64), (DRIVE, 30.0), (TONE, 8000.0)],
        ),
    ];

    for (name, settings) in demos {
        let mut runner = Runner::<Template>::new(RATE, 512);
        for &(index, plain) in settings {
            runner.params[index] = Template::PARAMS[index].normalized(plain);
        }
        let wet = runner.render(std::slice::from_ref(&dry), dry.len(), 512);
        let both: Vec<Vec<f32>> = wet
            .iter()
            .map(|ch| dry.iter().chain(ch).copied().collect())
            .collect();
        let file = PathBuf::from(&out).join(format!("{name}.wav"));
        wav::write(&file, RATE as u32, &both)?;
        println!("{}", file.display());
    }
    Ok(())
}
