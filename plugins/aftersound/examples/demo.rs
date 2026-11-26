//! Renders the demos someone listens to before trusting the tests:
//! `cargo run -p aftersound --example aftersound-demo -- OUT_DIR` writes
//! one WAV a setting worth hearing. Each plays the same phrase twice:
//! first on strings tuned alike and struck alike, which never leave the
//! middle, then with the settings that make the aftersound.

use aftersound::{AFTER, Aftersound, BRIGHT, MISTUNE, PROMPT, RELEASE, STRIKE};
use kit::{Event, Plugin, Runner};
use kit_test::wav;
use std::path::PathBuf;

const RATE: f32 = 48_000.0;
const BLOCK: usize = 512;

// Seconds from the start, key, velocity, seconds held.
type Note = (f32, u8, u8, f32);

// A slow phrase that leaves room for each note to bloom, then a chord
// left to ring.
const PHRASE: &[Note] = &[
    (0.0, 45, 96, 2.8),
    (0.0, 57, 90, 2.8),
    (3.0, 64, 84, 1.8),
    (5.0, 60, 80, 1.8),
    (7.0, 41, 100, 5.0),
    (7.0, 53, 92, 5.0),
    (7.0, 57, 88, 5.0),
    (7.0, 64, 84, 5.0),
];
const LENGTH: f32 = 14.0;

fn render(settings: &[(usize, f64)]) -> Vec<Vec<f32>> {
    let mut runner = Runner::<Aftersound>::new(RATE, BLOCK);
    for &(index, plain) in settings {
        runner.params[index] = Aftersound::PARAMS[index].normalized(plain);
    }
    let mut events: Vec<(usize, [u8; 3])> = PHRASE
        .iter()
        .flat_map(|&(at, key, velocity, held)| {
            let on = ((at * RATE) as usize, [0x90, key, velocity]);
            let off = (((at + held) * RATE) as usize, [0x80, key, 0]);
            [on, off]
        })
        .collect();
    events.sort_by_key(|e| e.0);
    let frames = (LENGTH * RATE) as usize;
    let mut out: Vec<Vec<f32>> = (0..2).map(|_| Vec::with_capacity(frames)).collect();
    let mut start = 0;
    while start < frames {
        let n = (frames - start).min(BLOCK);
        for &(at, data) in events.iter().filter(|e| (start..start + n).contains(&e.0)) {
            runner.push(Event {
                offset: (at - start) as u32,
                data,
            });
        }
        let piece = runner.render(&[], n, n);
        out[0].extend_from_slice(&piece[0]);
        out[1].extend_from_slice(&piece[1]);
        start += n;
    }
    out
}

fn main() -> std::io::Result<()> {
    let out = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "target/demos/aftersound".into());
    let plain = render(&[(MISTUNE, 0.0), (STRIKE, 0.0)]);
    let demos: [(&str, &[(usize, f64)]); 4] = [
        ("bloom", &[]),
        (
            "long-aftersound",
            &[(PROMPT, 0.3), (AFTER, 25.0), (MISTUNE, 0.5), (RELEASE, 1.5)],
        ),
        ("beating", &[(MISTUNE, 2.5), (PROMPT, 1.2), (BRIGHT, 70.0)]),
        (
            "off-centre",
            &[(STRIKE, 60.0), (MISTUNE, 0.4), (BRIGHT, 35.0)],
        ),
    ];

    for (name, settings) in demos {
        let wet = render(settings);
        let both: Vec<Vec<f32>> = (0..2)
            .map(|ch| plain[ch].iter().chain(&wet[ch]).copied().collect())
            .collect();
        let file = PathBuf::from(&out).join(format!("{name}.wav"));
        wav::write(&file, RATE as u32, &both)?;
        println!("{}", file.display());
    }
    Ok(())
}
