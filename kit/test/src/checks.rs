//! What every plugin is held to, whatever it does. `suite!` turns each
//! of these into a test; each panics with what is wrong.

use crate::budget::{self, allocations, cost};
use crate::signals;
use kit::{Event, Plugin, Role, Runner, Spec, Transport};

const RATE: f32 = 48_000.0;

// How a plugin is driven: an effect hears the signal, an instrument is
// played the phrase (and hears the signal too, if it has an input).
fn notes<P: Plugin>(rate: f32) -> Vec<Event> {
    match P::INFO.midi_in {
        true => signals::played(rate),
        false => Vec::new(),
    }
}

// Runs `audio` through `runner` in blocks of `block` with `notes`
// landing where they fall; the first output channel comes back.
fn play<P: Plugin>(
    runner: &mut Runner<P>,
    audio: &[f32],
    notes: &[Event],
    block: usize,
) -> Vec<f32> {
    let mut out = vec![0.0; audio.len()];
    let mut start = 0;
    while start < audio.len() {
        let n = (audio.len() - start).min(block);
        let here = start as u32..(start + n) as u32;
        for e in notes.iter().filter(|e| here.contains(&e.offset)) {
            runner.push(Event {
                offset: e.offset - start as u32,
                ..*e
            });
        }
        runner.run(
            n,
            Transport::default(),
            |_, s, into| into.copy_from_slice(&audio[start + s..start + s + into.len()]),
            |ch, s, from| {
                if ch == 0 {
                    out[start + s..start + s + from.len()].copy_from_slice(from);
                }
            },
            |_| {},
        );
        start += n;
    }
    out
}

// Every setting at `at` of its travel, snapped to a value it can hold.
fn all_at<P: Plugin>(mut at: impl FnMut(usize) -> f64) -> Vec<f64> {
    P::PARAMS
        .iter()
        .enumerate()
        .map(|(i, p)| p.normalized(p.plain(at(i))))
        .collect()
}

/// The spec is one a host can load and a later build can keep.
pub fn spec_is_sound<P: Plugin>() {
    // A host files a plugin by its kind: unsaid, Cubase has it under
    // "Other" with everything else that did not say.
    let kind = P::CATEGORY;
    assert!(
        kind != kit::Category::Other,
        "say what kind of plugin this is: `const CATEGORY` in its `impl Plugin`"
    );
    assert_eq!(
        kind.plays(),
        P::INFO.role == kit::Role::Instrument,
        "{kind:?} is not a kind of {:?}",
        P::INFO.role
    );
    let problems = Spec::of::<P>().problems();
    assert!(problems.is_empty(), "{}", problems.join("\n"));
    for p in P::PARAMS.iter().filter(|p| !p.read_only()) {
        for norm in [0.0, 0.25, 0.5, 1.0] {
            let text = p.display(norm);
            let back = p.parse(&text).map(|n| p.plain(n));
            let want = p.plain(norm);
            let close = back.is_some_and(|v| (v - want).abs() <= (want.abs() * 0.01).max(0.01));
            assert!(
                close,
                "{}: shows {text:?}, reads it back as {back:?}",
                p.key
            );
        }
    }
}

/// Whatever the settings and whatever it hears, the output is a
/// number, and not one that would blow a speaker.
pub fn output_is_finite<P: Plugin>() {
    let mut wild = signals::noise(7);
    let settings: Vec<(&str, Vec<f64>)> = vec![
        ("the defaults", Spec::of::<P>().defaults()),
        ("everything down", all_at::<P>(|_| 0.0)),
        ("everything up", all_at::<P>(|_| 1.0)),
        ("a mix", all_at::<P>(|i| (i as f64 * 0.37).fract())),
        ("another", all_at::<P>(|_| (wild() + 0.5) as f64)),
    ];
    let signals: [(&str, Vec<f32>); 4] = [
        ("silence", vec![0.0; RATE as usize / 2]),
        ("a voice", signals::sung(RATE)),
        ("full-scale noise", signals::white(RATE, 0.5)),
        ("an impulse", signals::impulse(RATE, 0.5)),
    ];

    for (which, values) in &settings {
        for (name, audio) in &signals {
            for block in [1, 64, 480] {
                let mut runner = Runner::<P>::new(RATE, block);
                runner.params.copy_from_slice(values);
                let out = play(&mut runner, audio, &notes::<P>(RATE), block);
                let worst = out.iter().fold(0.0f32, |m, s| m.max(s.abs()));
                let finite = out.iter().all(|s| s.is_finite());
                assert!(
                    finite && worst < 16.0,
                    "{which}, {name}, blocks of {block}: peak {worst}"
                );
            }
        }
    }
}

/// At its defaults it does something: an effect given a voice, or an
/// instrument played a phrase, is heard.
pub fn sounds_at_its_defaults<P: Plugin>() {
    let mut runner = Runner::<P>::new(RATE, 256);
    let out = play(&mut runner, &signals::sung(RATE), &notes::<P>(RATE), 256);
    let peak = out.iter().fold(0.0f32, |m, s| m.max(s.abs()));
    let what = match P::INFO.role {
        Role::Effect => "a voice through it",
        Role::Instrument => "a phrase played on it",
    };
    assert!(peak > 0.001, "{what} came out at a peak of {peak}");
}

/// The same input gives the same output, sample for sample: anything
/// random in it starts from the same seed every time.
pub fn is_deterministic<P: Plugin>() {
    let audio = signals::sung(RATE);
    let once = play(
        &mut Runner::<P>::new(RATE, 128),
        &audio,
        &notes::<P>(RATE),
        128,
    );
    let twice = play(
        &mut Runner::<P>::new(RATE, 128),
        &audio,
        &notes::<P>(RATE),
        128,
    );
    let differs = once
        .iter()
        .zip(&twice)
        .position(|(a, b)| a.to_bits() != b.to_bits());
    assert_eq!(differs, None, "two runs part at this sample");
}

/// The host's block size is not heard: a bounce at one buffer size
/// matches a bounce at another.
pub fn block_size_is_not_heard<P: Plugin>() {
    let audio = signals::sung(RATE);
    let small = play(
        &mut Runner::<P>::new(RATE, 32),
        &audio,
        &notes::<P>(RATE),
        32,
    );
    let large = play(
        &mut Runner::<P>::new(RATE, 1024),
        &audio,
        &notes::<P>(RATE),
        1024,
    );
    let worst = small
        .iter()
        .zip(&large)
        .map(|(a, b)| (a - b).abs())
        .fold(0.0f32, f32::max);
    assert!(
        worst < 1e-3,
        "blocks of 32 and of 1024 differ by up to {worst}"
    );
}

/// `process` never allocates or frees, with settings moving and notes
/// arriving. Needs `Counting` as the test binary's allocator, which
/// `suite!` sets.
pub fn never_allocates<P: Plugin>() {
    let audio = signals::sung(RATE);
    let notes = notes::<P>(RATE);
    for block in [64, 512] {
        let mut runner = Runner::<P>::new(RATE, block);
        // Past the first blocks, where a lazy start would show.
        play(&mut runner, &audio[..block * 4], &[], block);
        let low = all_at::<P>(|_| 0.1);
        let high = all_at::<P>(|_| 0.9);
        let mut out = vec![0.0; audio.len()];
        let count = allocations(|| {
            let mut start = 0;
            let mut turn = 0;
            while start + block <= audio.len() {
                let values = if turn % 2 == 0 { &low } else { &high };
                runner.params.copy_from_slice(values);
                let here = start as u32..(start + block) as u32;
                for e in notes.iter().filter(|e| here.contains(&e.offset)) {
                    runner.push(Event {
                        offset: e.offset - start as u32,
                        ..*e
                    });
                }
                runner.run(
                    block,
                    Transport::default(),
                    |_, s, into| into.copy_from_slice(&audio[start + s..start + s + into.len()]),
                    |_, s, from| out[start + s..start + s + from.len()].copy_from_slice(from),
                    |_| {},
                );
                start += block;
                turn += 1;
            }
        });
        assert_eq!(count, 0, "allocations or frees in blocks of {block}");
    }
}

/// One instance costs under a tenth of a core on average, and no block
/// takes over half its own length, at any rate and block size.
pub fn is_within_budget<P: Plugin>() {
    let over = budget::over(|rate, block| {
        let audio = signals::sung(rate);
        let notes = notes::<P>(rate);
        // Where in the signal the instance being timed has got to.
        let at = std::cell::Cell::new(0usize);
        cost(
            rate,
            block,
            &audio,
            || {
                at.set(0);
                Runner::<P>::new(rate, block)
            },
            |runner, chunk| {
                let start = at.get();
                let here = start as u32..(start + chunk.len()) as u32;
                for e in notes.iter().filter(|e| here.contains(&e.offset)) {
                    runner.push(Event {
                        offset: e.offset - start as u32,
                        ..*e
                    });
                }
                runner.run(
                    chunk.len(),
                    Transport::default(),
                    |_, s, into| into.copy_from_slice(&chunk[s..s + into.len()]),
                    |_, _, _| {},
                    |_| {},
                );
                at.set(start + chunk.len());
            },
        )
    });
    assert!(
        over.is_empty(),
        "over the realtime budget:\n{}",
        over.join("\n")
    );
}

/// Loaded through each format's own front door, it describes itself as
/// its spec does and its state survives a save and a load; and what
/// comes out of each is what comes out of the plugin itself.
pub fn loads_in_every_format<P: Plugin>() {
    kit_vst3::testing::conforms::<P>();
    kit_clap::testing::conforms::<P>();

    let audio = signals::sung(RATE);
    let frames = RATE as usize / 2;
    let input = vec![audio[..frames].to_vec()];
    let want = Runner::<P>::new(RATE, 256).render(&input, frames, 256);
    let vst3 = kit_vst3::testing::Host::load::<P>(RATE, 256).render(&input, frames, 256);
    let clap = kit_clap::testing::Host::load::<P>(RATE, 256).render(&input, frames, 256);
    assert!(
        vst3 == want,
        "the VST3 sounds different from the plugin itself"
    );
    assert!(
        clap == want,
        "the CLAP sounds different from the plugin itself"
    );
}
