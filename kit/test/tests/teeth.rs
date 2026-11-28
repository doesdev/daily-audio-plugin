//! The checks must fail a plugin that breaks them, or they are worth
//! nothing: a deliberately bad plugin for each, and a good one that
//! passes them all.

use kit::{Block, Info, Param, Plugin, Role, range};
use kit_test::checks;

#[global_allocator]
static ALLOC: kit_test::Counting = kit_test::Counting;

const fn info(clap_id: &'static str, n: u32) -> Info {
    Info {
        name: "Bad",
        vendor: "kit",
        url: "",
        description: "",
        version: "0.0.0",
        clap_id,
        vst3_processor: [n, 1, 1, 1],
        vst3_controller: [n, 2, 2, 2],
        role: Role::Effect,
        audio_in: 2,
        audio_out: 2,
        midi_in: false,
        midi_out: false,
        size: (400, 300),
        min_size: (200, 100),
    }
}

const GAIN: Param = range(0, "gain", "Gain", "", (0.0, 2.0, 1.0), "");

// How each plugin below goes wrong, in its `process`.
#[derive(Clone, Copy, PartialEq)]
enum Fault {
    None,
    Allocates,
    NotANumber,
    HearsTheBlock,
    Random,
    Silent,
}

struct Bad<const F: u8> {
    scratch: Vec<f32>,
    calls: u32,
}

fn fault(f: u8) -> Fault {
    [
        Fault::None,
        Fault::Allocates,
        Fault::NotANumber,
        Fault::HearsTheBlock,
        Fault::Random,
        Fault::Silent,
    ][f as usize]
}

impl<const F: u8> Plugin for Bad<F> {
    const CATEGORY: kit::Category = kit::Category::Tools;
    const INFO: &'static Info = &info("dev.kit.bad", F as u32 + 100);
    const PARAMS: &'static [Param] = &[GAIN];

    fn new() -> Self {
        Self {
            scratch: Vec::new(),
            calls: 0,
        }
    }

    fn prepare(&mut self, _rate: f32, _max_block: usize) {}

    fn process(&mut self, block: &mut Block) {
        let fault = fault(F);
        self.calls += 1;
        if fault == Fault::Allocates {
            self.scratch = vec![0.0; block.frames()];
        }
        let frames = block.frames();
        let gain = match fault {
            Fault::NotANumber if block.value(0) > 1.9 => f32::NAN,
            Fault::HearsTheBlock => 1.0 / frames as f32,
            Fault::Random => {
                let here = &self.calls as *const u32 as usize;
                1.0 + (here % 97) as f32 * 1e-3 * (std::process::id() % 7) as f32
                    + std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map_or(0.0, |d| d.subsec_nanos() as f32 * 1e-12)
            }
            Fault::Silent => 0.0,
            _ => block.value(0),
        };
        for ch in 0..block.outputs() {
            let (input, output) = block.io(ch);
            output
                .iter_mut()
                .zip(input)
                .for_each(|(o, i)| *o = i * gain);
        }
    }
}

type Good = Bad<0>;

#[test]
fn a_sound_plugin_passes_every_check() {
    checks::spec_is_sound::<Good>();
    checks::output_is_finite::<Good>();
    checks::sounds_at_its_defaults::<Good>();
    checks::is_deterministic::<Good>();
    checks::block_size_is_not_heard::<Good>();
    checks::never_allocates::<Good>();
    checks::loads_in_every_format::<Good>();
}

#[test]
#[should_panic(expected = "allocations or frees")]
fn allocating_in_process_is_caught() {
    checks::never_allocates::<Bad<1>>();
}

#[test]
#[should_panic(expected = "everything up")]
fn a_setting_that_makes_nan_is_caught() {
    checks::output_is_finite::<Bad<2>>();
}

#[test]
#[should_panic(expected = "blocks of 32 and of 1024 differ")]
fn hearing_the_block_size_is_caught() {
    checks::block_size_is_not_heard::<Bad<3>>();
}

#[test]
#[should_panic(expected = "two runs part")]
fn randomness_without_a_seed_is_caught() {
    checks::is_deterministic::<Bad<4>>();
}

#[test]
#[should_panic(expected = "came out at a peak of 0")]
fn a_plugin_that_makes_no_sound_is_caught() {
    checks::sounds_at_its_defaults::<Bad<5>>();
}

// A spec no host could load: an id used twice and a default out of
// range.
struct Muddled;

impl Plugin for Muddled {
    const CATEGORY: kit::Category = kit::Category::Tools;
    const INFO: &'static Info = &info("muddled", 1);
    const PARAMS: &'static [Param] = &[GAIN, range(0, "tone", "Tone", "", (0.0, 1.0, 5.0), "")];

    fn new() -> Self {
        Self
    }

    fn prepare(&mut self, _rate: f32, _max_block: usize) {}

    fn process(&mut self, _block: &mut Block) {}
}

#[test]
fn a_muddled_spec_is_caught_for_each_thing_wrong() {
    let problems = kit::Spec::of::<Muddled>().problems();
    let wanted = [
        "tone: id 0 is used twice",
        "tone: default 5 is not a value it can hold",
        "CLAP id \"muddled\" is not reverse-domain",
    ];

    for want in wanted {
        assert!(problems.iter().any(|p| p == want), "{want}: {problems:?}");
    }
    assert_eq!(problems.len(), wanted.len(), "{problems:?}");
}

// A plugin that does not say what kind it is, and one that says an
// instrument's kind while being an effect.
struct Unsaid;
struct Miscast;

impl Plugin for Unsaid {
    const INFO: &'static Info = &info("dev.kit.unsaid", 300);
    const PARAMS: &'static [Param] = &[GAIN];

    fn new() -> Self {
        Self
    }

    fn prepare(&mut self, _rate: f32, _max_block: usize) {}

    fn process(&mut self, _block: &mut Block) {}
}

impl Plugin for Miscast {
    const CATEGORY: kit::Category = kit::Category::Synth;
    const INFO: &'static Info = &info("dev.kit.miscast", 301);
    const PARAMS: &'static [Param] = &[GAIN];

    fn new() -> Self {
        Self
    }

    fn prepare(&mut self, _rate: f32, _max_block: usize) {}

    fn process(&mut self, _block: &mut Block) {}
}

#[test]
#[should_panic(expected = "say what kind of plugin this is")]
fn a_plugin_that_does_not_say_its_kind_is_caught() {
    checks::spec_is_sound::<Unsaid>();
}

#[test]
#[should_panic(expected = "Synth is not a kind of Effect")]
fn an_effect_filed_as_an_instrument_is_caught() {
    checks::spec_is_sound::<Miscast>();
}

// A plugin that works harder once the sound has gone than while it
// played, the way a ringing filter left to denormals does. It does
// the extra work itself, since what a denormal costs is the
// processor's business: on some it is eight times, on the machines
// CI runs on next to nothing, and a check must bite on both.
struct Restless {
    held: f32,
}

impl Plugin for Restless {
    const CATEGORY: kit::Category = kit::Category::Tools;
    const INFO: &'static Info = &info("dev.kit.restless", 302);
    const PARAMS: &'static [Param] = &[GAIN];

    fn new() -> Self {
        Self { held: 0.0 }
    }

    fn prepare(&mut self, _rate: f32, _max_block: usize) {}

    fn process(&mut self, block: &mut Block) {
        for ch in 0..block.outputs() {
            let (input, output) = block.io(ch);
            for (o, i) in output.iter_mut().zip(input) {
                self.held = self.held * 0.97 + *i;
                let turns = if self.held.abs() < 1e-6 { 400 } else { 20 };
                let mut sum = 0.0f32;
                for n in 0..turns {
                    sum += std::hint::black_box(self.held + n as f32).sin();
                }
                *o = self.held + sum * 0.0;
            }
        }
    }
}

#[test]
#[should_panic(expected = "silence after a sound costs")]
fn a_tail_that_costs_more_than_the_sound_is_caught() {
    checks::a_tail_costs_no_more_than_the_sound::<Restless>();
}

// The other half: that nothing a plugin does in the runner meets a
// denormal at all, which no clock is needed to see.
struct Tiny;

impl Plugin for Tiny {
    const CATEGORY: kit::Category = kit::Category::Tools;
    const INFO: &'static Info = &info("dev.kit.tiny", 303);
    const PARAMS: &'static [Param] = &[GAIN];

    fn new() -> Self {
        Self
    }

    fn prepare(&mut self, _rate: f32, _max_block: usize) {}

    fn process(&mut self, block: &mut Block) {
        // Half the smallest whole float is a denormal, or nothing.
        let half = std::hint::black_box(f32::MIN_POSITIVE) * std::hint::black_box(0.5);
        for ch in 0..block.outputs() {
            let (_, output) = block.io(ch);
            output.fill(if half == 0.0 { 0.0 } else { 1.0 });
        }
    }
}

#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
#[test]
fn a_plugin_in_the_runner_never_meets_a_denormal() {
    let mut runner = kit::Runner::<Tiny>::new(48_000.0, 64);
    let mut met = 0.0f32;
    runner.run(
        64,
        kit::Transport::default(),
        |_, _, into| into.fill(0.0),
        |_, _, out| met = met.max(out.iter().fold(0.0, |a, b| a.max(*b))),
        |_| {},
    );
    assert_eq!(met, 0.0);
}
