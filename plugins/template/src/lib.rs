//! Template: drive into a shaper, a tone filter after it, and a mix
//! back against the dry signal. It is not shipped. It is the plugin
//! `npm run new` copies, and the proof that the kit works end to end:
//! spec, sound, editor, tests, screenshots and manual.

use kit::level::{db_to_gain, peak, to_fraction};
use kit::{Asset, Block, Info, Param, Plugin, Role, list, meter, range};
use kit_dsp::{Match, Smooth, Svf};

// Indexes into PARAMS, for `Block`.
pub const DRIVE: usize = 0;
pub const SHAPE: usize = 1;
pub const TONE: usize = 2;
pub const MIX: usize = 3;
pub const OUTPUT: usize = 4;
pub const BYPASS: usize = 5;
pub const MATCH: usize = 6;
// Indexes into METERS.
pub const LEVEL: usize = 0;
pub const INPUT: usize = 1;

pub const SOFT: usize = 0;
pub const HARD: usize = 1;
pub const FOLD: usize = 2;

// How fast a moved setting is followed, in ms.
const EASE_MS: f32 = 20.0;

pub struct Template {
    rate: f32,
    filters: [Svf; 2],
    drive: Smooth,
    tone: Smooth,
    mix: Smooth,
    matcher: Match,
    output: Smooth,
}

/// One sample through the shaper `shape` names.
pub fn shaped(shape: usize, x: f32) -> f32 {
    match shape {
        SOFT => x.tanh(),
        HARD => x.clamp(-1.0, 1.0),
        // Folds back from the rails rather than stopping at them.
        _ => {
            let t = (x + 1.0).rem_euclid(4.0);
            if t < 2.0 { t - 1.0 } else { 3.0 - t }
        }
    }
}

impl Plugin for Template {
    // Where a host files it: say what the plugin is.
    const CATEGORY: kit::Category = kit::Category::Distortion;
    const INFO: &'static Info = &Info {
        name: "Template",
        vendor: kit::VENDOR,
        url: "https://github.com/doesdev/daily-audio-plugin",
        description: "Drive, tone and mix",
        version: env!("CARGO_PKG_VERSION"),
        // Fixed for good: hosts save projects by these three.
        clap_id: "dev.doesdev.template",
        vst3_processor: [0x9C1E_52A7, 0x4B3D_4F10, 0x8E6A_2D91, 0x5C07_B3F4],
        vst3_controller: [0x2F8B_6D03, 0xA94C_4E57, 0xB1D2_7A68, 0x3E95_C0D1],
        role: Role::Effect,
        audio_in: 2,
        audio_out: 2,
        midi_in: false,
        midi_out: false,
        size: (820, 500),
        min_size: (615, 375),
    };

    // The ids are fixed for good too. A new setting takes the next id
    // and goes wherever it reads best; none is ever reused.
    const PARAMS: &'static [Param] = &[
        range(0, "drive", "Drive", "Shape", (0.0, 36.0, 12.0), "dB")
            .decimals(1)
            .about("How hard the signal is pushed into the shaper"),
        list(
            1,
            "shape",
            "Shape",
            "Shape",
            &["Soft", "Hard", "Fold"],
            SOFT,
        )
        .about("How the shaper bends what goes past its limit"),
        range(2, "tone", "Tone", "Colour", (200.0, 16_000.0, 6000.0), "Hz")
            .skew(3.0)
            .decimals(0)
            .about("Where the low-pass after the shaper starts to cut"),
        kit::mix(3),
        range(4, "output", "Output", "Output", (-24.0, 12.0, -6.0), "dB")
            .decimals(1)
            .about("Level after everything else"),
        // Ids 5 and 6 are the meters'.
        kit::bypass(7),
        kit::level_match(8),
    ];

    const METERS: &'static [Param] = &[meter(5, "level", "Out"), meter(6, "input", "In")];

    const ASSETS: &'static [Asset] = &[
        ("/main.js", "text/javascript", include_str!("../ui/main.js")),
        ("/style.css", "text/css", include_str!("../ui/style.css")),
    ];

    fn new() -> Self {
        let rate = 48_000.0;
        Self {
            rate,
            filters: [Svf::new(rate); 2],
            drive: Smooth::new(0.0, EASE_MS, rate),
            tone: Smooth::new(0.0, EASE_MS, rate),
            mix: Smooth::new(0.0, EASE_MS, rate),
            matcher: Match::new(rate),
            output: Smooth::new(0.0, EASE_MS, rate),
        }
    }

    fn prepare(&mut self, rate: f32, _max_block: usize) {
        let at = |index: usize| Self::PARAMS[index].default as f32;
        self.rate = rate;
        self.filters = [Svf::new(rate); 2];
        self.drive = Smooth::new(db_to_gain(at(DRIVE)), EASE_MS, rate);
        self.tone = Smooth::new(at(TONE), EASE_MS, rate);
        self.mix = Smooth::new(at(MIX) / 100.0, EASE_MS, rate);
        self.matcher = Match::new(rate);
        self.output = Smooth::new(db_to_gain(at(OUTPUT)), EASE_MS, rate);
    }

    fn reset(&mut self) {
        self.filters.iter_mut().for_each(Svf::reset);
    }

    fn process(&mut self, block: &mut Block) {
        let drive = db_to_gain(block.value(DRIVE));
        let tone = block.value(TONE);
        let mix = block.value(MIX) / 100.0;
        let output = db_to_gain(block.value(OUTPUT));
        let shape = block.choice(SHAPE);
        let matching = block.choice(MATCH) == 1;

        let mut loudest = 0.0f32;
        let mut heard = 0.0f32;
        for ch in 0..block.inputs() {
            heard = heard.max(peak(block.input(ch)));
        }
        for i in 0..block.frames() {
            // Each setting eased once a sample, the same for both sides.
            let (drive, mix) = (self.drive.next(drive), self.mix.next(mix));
            let (tone, output) = (self.tone.next(tone), self.output.next(output));
            let (mut heard_now, mut made) = (0.0, 0.0);
            let mut mixed = [0.0f32; 2];
            let sides = block.outputs().min(2);
            for (ch, mixed) in mixed.iter_mut().enumerate().take(sides) {
                let dry = block.input(ch).get(i).copied().unwrap_or(0.0);
                let filter = &mut self.filters[ch];
                filter.set(tone, 0.707);
                let wet = filter.next(shaped(shape, dry * drive)).low;
                *mixed = dry + (wet - dry) * mix;
                heard_now += dry * dry;
                made += *mixed * *mixed;
            }
            // Matched to what came in before the output trim, which is
            // then the person's own to set on top.
            let gain = self.matcher.next(heard_now, made, matching) * output;
            for (ch, mixed) in mixed.iter().enumerate().take(sides) {
                block.output(ch)[i] = mixed * gain;
            }
        }
        for ch in 0..block.outputs() {
            loudest = loudest.max(peak(block.output(ch)));
        }
        block.meter(LEVEL, to_fraction(loudest));
        block.meter(INPUT, to_fraction(heard));
    }
}

kit_vst3::export!(Template);
kit_clap::export!(Template);
