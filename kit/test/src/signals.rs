//! Test signals, the same on every machine: nothing here is random
//! beyond a fixed seed.

use kit::Event;
use std::f32::consts::TAU;

/// Noise from -0.5 to 0.5, the same sequence for the same seed.
pub fn noise(seed: u32) -> impl FnMut() -> f32 {
    let mut seed = seed.max(1);
    move || {
        seed ^= seed << 13;
        seed ^= seed >> 17;
        seed ^= seed << 5;
        seed as f32 / u32::MAX as f32 - 0.5
    }
}

/// `seconds` of a steady tone at `hz`, peaking at `gain`.
pub fn sine(rate: f32, seconds: f32, hz: f32, gain: f32) -> Vec<f32> {
    (0..(rate * seconds) as usize)
        .map(|i| gain * (TAU * hz * i as f32 / rate).sin())
        .collect()
}

/// `seconds` of full-scale white noise.
pub fn white(rate: f32, seconds: f32) -> Vec<f32> {
    let mut next = noise(0x9e37_79b9);
    (0..(rate * seconds) as usize)
        .map(|_| 2.0 * next())
        .collect()
}

/// One full-scale sample, then silence.
pub fn impulse(rate: f32, seconds: f32) -> Vec<f32> {
    let mut out = vec![0.0; (rate * seconds) as usize];
    out[0] = 1.0;
    out
}

/// Sung phrases: a few notes with vibrato and harmonics, gaps of
/// silence between them, a little noise throughout. Two seconds of the
/// kind of thing a plugin meets on a track.
pub fn sung(rate: f32) -> Vec<f32> {
    let notes = [220.0, 330.0, 262.0, 440.0];
    let (note_len, gap) = (0.4, 0.1);
    let mut hiss = noise(0x2545_f491);
    let mut phase = 0.0f32;

    (0..(rate * 2.0) as usize)
        .map(|i| {
            let t = i as f32 / rate;
            let slot = (t / (note_len + gap)) as usize;
            let into = t - slot as f32 * (note_len + gap);
            let hiss = 0.002 * hiss();
            if into >= note_len {
                return hiss;
            }

            let vibrato = 2f32.powf(0.3 * (TAU * 5.0 * t).sin() / 12.0);
            phase = (phase + notes[slot % notes.len()] * vibrato / rate).fract();
            let x = TAU * phase;
            0.3 * (x.sin() + 0.5 * (2.0 * x).sin() + 0.25 * (3.0 * x).sin()) / 1.75 + hiss
        })
        .collect()
}

/// A played phrase for an instrument: a note every half second for two
/// seconds, each held 0.4 s, a chord on the last. Offsets are samples
/// from the start, in order.
pub fn played(rate: f32) -> Vec<Event> {
    let at = |seconds: f32| (rate * seconds) as u32;
    let mut out = Vec::new();
    for (i, key) in [57u8, 60, 64].into_iter().enumerate() {
        let start = i as f32 * 0.5;
        out.push(Event {
            offset: at(start),
            data: [0x90, key, 100],
        });
        out.push(Event {
            offset: at(start + 0.4),
            data: [0x80, key, 0],
        });
    }
    for key in [57u8, 60, 64] {
        out.push(Event {
            offset: at(1.5),
            data: [0x90, key, 90],
        });
    }
    for key in [57u8, 60, 64] {
        out.push(Event {
            offset: at(1.9),
            data: [0x80, key, 0],
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn signals_are_the_same_every_time_and_in_range() {
        let rate = 48_000.0;
        assert_eq!(sung(rate), sung(rate));
        assert_eq!(white(rate, 0.1), white(rate, 0.1));

        for (name, signal) in [("sung", sung(rate)), ("white", white(rate, 0.5))] {
            let peak = signal.iter().fold(0.0f32, |m, s| m.max(s.abs()));
            assert!(peak > 0.2 && peak <= 1.0, "{name}: {peak}");
        }
        assert_eq!(sung(rate).len(), 96_000);
        assert_eq!(impulse(rate, 0.01).iter().sum::<f32>(), 1.0);
        let notes = played(rate);
        assert!(
            notes.windows(2).all(|w| w[0].offset <= w[1].offset),
            "in order"
        );
    }
}
