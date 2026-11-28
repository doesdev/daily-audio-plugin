//! Ironside: the sides are wound on a core that the middle fills. The
//! middle goes through a transformer; the sides go through a choke on
//! the same core, across their path. With the core empty the choke
//! holds back only what is below Corner. As the middle's flux fills
//! the core the choke's inductance falls away and the sides' corner
//! climbs toward Ceiling, on each swing of the bass. A saturable
//! reactor, between the two halves of a stereo sound.

use kit::level::{db_to_gain, peak, to_fraction};
use kit::{Asset, Block, Info, Param, Plugin, Rate, Role, meter, quality, range};
use kit_dsp::{
    Bend, Delay, IronShape, Match, MidSide, Need, Oversampled, Smooth, Transformer, pole,
};

// Indexes into PARAMS, for `Block`.
pub const FILL: usize = 0;
pub const CORNER: usize = 1;
pub const CEILING: usize = 2;
pub const RECOVER: usize = 3;
pub const MIX: usize = 4;
pub const OUTPUT: usize = 5;
pub const BYPASS: usize = 6;
pub const MATCH: usize = 7;
pub const BALANCE: usize = 8;
pub const LISTEN: usize = 9;
pub const KEY: usize = 10;
pub const IRON_DRIVE: usize = 11;
pub const IRON_SIZE: usize = 12;
pub const IRON_MATERIAL: usize = 13;
pub const IRON_SOURCE: usize = 14;
pub const IRON_GRIP: usize = 15;
pub const IRON_TOP: usize = 16;
pub const IRON_RING: usize = 17;
pub const OVERSAMPLING: usize = 18;
pub const RENDER: usize = 19;
// Indexes into METERS.
pub const INPUT: usize = 0;
pub const LEVEL: usize = 1;
pub const FULL: usize = 2;

// How fast a moved setting is followed, in ms.
const EASE_MS: f32 = 20.0;

/// How hard the sides' winding is pushed at Fill 100%: 24 dB. At 50%
/// a bass at -12 dB at the core's size all but shuts it.
pub const MOST_FILL: f32 = 16.0;

// The fill meter holds its peak this long, so the picture's corner is
// readable at the rate an editor redraws.
const FULL_HOLD_MS: f32 = 60.0;

/// Where the sides' corner stands, in hertz, for a core `open` (one
/// empty, toward nought full) with the corner at `rest` with it
/// empty and never above `ceiling`. A choke's corner is its source's
/// resistance over its inductance, and the inductance is the core's
/// slope: half as open, twice as high.
pub fn corner_of(rest: f32, ceiling: f32, open: f32) -> f32 {
    (rest / open.max(1e-6)).min(ceiling.max(rest))
}

/// The choke the sides pass through: a high-pass whose corner is set
/// every sample by how open the core is. It runs at whatever rate
/// keeps what its moving corner adds from folding back.
#[derive(Clone, Copy)]
pub struct Choke {
    rate: f32,
    rest: f32,
    ceiling: f32,
    // What the choke has let through below the corner so far.
    low: f32,
}

impl Choke {
    pub fn new(rate: f32) -> Self {
        Self {
            rate,
            rest: 30.0,
            ceiling: 1200.0,
            low: 0.0,
        }
    }

    pub fn set(&mut self, rest: f32, ceiling: f32) {
        (self.rest, self.ceiling) = (rest, ceiling);
    }

    pub fn reset(&mut self) {
        self.low = 0.0;
    }

    /// One sample of the sides with the core `open`.
    #[inline]
    pub fn next(&mut self, x: f32, open: f32) -> f32 {
        let corner = corner_of(self.rest, self.ceiling, open);
        // A one-pole high-pass in its trapezoidal form, which stays
        // what it is however fast its corner is moved.
        let g = (std::f32::consts::PI * corner / self.rate).min(1.5).tan();
        let v = (x - self.low) * g / (1.0 + g);
        let low = v + self.low;
        self.low = low + v;
        x - low
    }
}

impl Bend for Choke {
    fn retune(&mut self, rate: f32) {
        self.rate = rate;
    }
}

pub struct Ironside {
    rate: f32,
    mix: Smooth,
    output: Smooth,
    matcher: Match,
    halves: MidSide,
    rendering: bool,
    // The middle's own core, heard, and the same core again read by
    // the sides' winding at the plain rate.
    heard: Oversampled<Transformer>,
    core: Transformer,
    choke: Oversampled<Choke>,
    needs: [Need; 2],
    // How open the core is held, letting go as Recover says.
    open: f32,
    full: f32,
    full_fall: f32,
    waiting: [Delay; 2],
}

impl Plugin for Ironside {
    // A transformer's colour on the stereo image: what it adds is what
    // the core takes away.
    const CATEGORY: kit::Category = kit::Category::Distortion;
    const INFO: &'static Info = &Info {
        name: "Ironside",
        vendor: kit::VENDOR,
        url: "https://github.com/doesdev/daily-audio-plugin",
        description: "The sides are wound on a core the middle fills",
        version: env!("CARGO_PKG_VERSION"),
        // Fixed for good: hosts save projects by these three.
        clap_id: "dev.doesdev.ironside",
        vst3_processor: [0xEBE9_D7AE, 0x55A8_F427, 0x4079_0B13, 0x15FF_34A6],
        vst3_controller: [0xCBA0_841C, 0x8F6C_F817, 0xA344_3E88, 0x7EC1_26F8],
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
        range(0, "fill", "Fill", "Core", (0.0, 100.0, 50.0), "%")
            .decimals(0)
            .about("How hard the middle's flux fills the core the sides are wound on"),
        range(1, "corner", "Corner", "Sides", (10.0, 300.0, 30.0), "Hz")
            .skew(2.0)
            .step(1.0)
            .decimals(0)
            .about("Where the sides' lows start to go with the core empty"),
        range(2, "ceiling", "Ceiling", "Sides", (0.2, 5.0, 1.2), "kHz")
            .skew(2.5)
            .step(0.1)
            .decimals(1)
            .about("The highest the middle can push the sides' corner, with the core full"),
        range(3, "recover", "Recover", "Core", (0.0, 500.0, 5.0), "ms")
            .skew(3.0)
            .step(0.1)
            .decimals(1)
            .about(
                "How long the core takes to let the sides back: at nought, every swing of the bass",
            ),
        kit::mix(4),
        range(5, "output", "Output", "Output", (-24.0, 12.0, 0.0), "dB")
            .decimals(1)
            .about("Level after everything else"),
        // Ids 6 to 8 are the meters'.
        kit::bypass(9),
        kit::level_match(10),
        kit::balance(11),
        kit::listen(12),
        kit::key(13),
        kit::iron_drive(14),
        kit::iron_size(15),
        kit::iron_material(16),
        kit::iron_source(17),
        kit::iron_grip(18),
        kit::iron_top(19),
        kit::iron_ring(20),
        kit::oversampling(21),
        kit::render_quality(22),
    ];

    // A sidechain: another track's bass can fill the core.
    const KEY_IN: u32 = 2;
    const METERS: &'static [Param] = &[
        meter(6, "input", "In"),
        meter(7, "level", "Out"),
        meter(8, "full", "Full").fraction(),
    ];

    const ASSETS: &'static [Asset] = &[
        ("/main.js", "text/javascript", include_str!("../ui/main.js")),
        ("/style.css", "text/css", include_str!("../ui/style.css")),
        ("/core.js", "text/javascript", include_str!("../ui/core.js")),
    ];

    fn new() -> Self {
        let rate = 48_000.0;
        Self {
            rate,
            mix: Smooth::new(1.0, EASE_MS, rate),
            output: Smooth::new(1.0, EASE_MS, rate),
            matcher: Match::new(rate),
            halves: MidSide::new(rate),
            rendering: false,
            heard: Oversampled::new(rate, false, Transformer::new(rate)),
            core: Transformer::new(rate),
            choke: Oversampled::new(rate, false, Choke::new(rate)),
            needs: [Need::new(rate); 2],
            open: 1.0,
            full: 0.0,
            full_fall: 0.0,
            waiting: std::array::from_fn(|_| Delay::new(1)),
        }
    }

    fn rendering(&mut self, on: bool) {
        self.rendering = on;
    }

    fn latency(&self) -> u32 {
        self.heard.delay() as u32
    }

    fn prepare(&mut self, rate: f32, _max_block: usize) {
        let at = |index: usize| Self::PARAMS[index].default as f32;
        self.rate = rate;
        self.mix = Smooth::new(at(MIX) / 100.0, EASE_MS, rate);
        self.output = Smooth::new(db_to_gain(at(OUTPUT)), EASE_MS, rate);
        self.matcher = Match::new(rate);
        self.halves = MidSide::new(rate);
        self.heard = Oversampled::new(rate, self.rendering, Transformer::new(rate));
        self.core = Transformer::new(rate);
        self.choke = Oversampled::new(rate, self.rendering, Choke::new(rate));
        self.needs = [Need::new(rate); 2];
        let late = self.heard.delay() + 1;
        self.waiting = std::array::from_fn(|_| Delay::new(late));
        self.open = 1.0;
        self.full = 0.0;
        self.full_fall = 1.0 / (FULL_HOLD_MS * 0.001 * rate);
    }

    fn reset(&mut self) {
        self.heard.reset(Transformer::reset);
        self.core.reset();
        self.choke.reset(Choke::reset);
        self.needs.iter_mut().for_each(Need::reset);
        self.waiting.iter_mut().for_each(Delay::reset);
        self.open = 1.0;
        self.full = 0.0;
    }

    fn process(&mut self, block: &mut Block) {
        let push = block.value(FILL) / 100.0 * MOST_FILL;
        let rest = block.value(CORNER);
        let ceiling = block.value(CEILING) * 1000.0;
        let recover = block.value(RECOVER);
        let mix = block.value(MIX) / 100.0;
        let output = db_to_gain(block.value(OUTPUT));
        let matching = block.choice(MATCH) == 1;
        let balance = block.value(BALANCE) / 100.0;
        let listen = block.choice(LISTEN);
        let keyed = block.on(KEY);
        let quality = quality(
            block.choice(OVERSAMPLING),
            block.choice(RENDER),
            self.rendering,
        );
        // The core is set once a block from the transformer window:
        // it is the same core heard on the middle and read by the
        // sides' winding.
        let shape = IronShape {
            drive: block.value(IRON_DRIVE) / 100.0,
            size: block.value(IRON_SIZE),
            iron: block.value(IRON_MATERIAL) / 100.0,
            source: block.value(IRON_SOURCE) / 100.0,
            grip: block.value(IRON_GRIP) / 100.0,
            top: block.value(IRON_TOP) / 100.0,
            ring: block.value(IRON_RING) / 100.0,
        };
        self.heard.each(|core| core.set(shape));
        self.core.set(shape);
        self.choke.each(|choke| choke.set(rest, ceiling));
        // Recover lets the core open again; it closes at once.
        let back = match recover > 0.0 {
            true => 1.0 - pole(recover, self.rate),
            false => 1.0,
        };
        let late = (self.heard.delay() + 1) as f32;

        let mut heard = 0.0f32;
        for ch in 0..block.inputs() {
            heard = heard.max(peak(block.input(ch)));
        }
        for i in 0..block.frames() {
            let (mix, output) = (self.mix.next(mix), self.output.next(output));
            let (left, right) = (sample(block, 0, i), sample(block, 1, i));
            let (mid, side) = ((left + right) * 0.5, (left - right) * 0.5);
            // What fills the core: the middle, or with the sidechain on
            // the middle of the track routed there, and then nothing
            // of this sound's own.
            let filler = match keyed {
                true => (key_sample(block, 0, i) + key_sample(block, 1, i)) * 0.5,
                false => mid,
            };
            self.core.next(filler);
            let open = self.core.opening(push);
            self.open = open.min(self.open + (1.0 - self.open) * back);
            self.full = (self.full - self.full_fall).max(1.0 - self.open);

            // The middle through its core, and the sides through the
            // choke, each at the rate that keeps what it adds from
            // folding back.
            let [mid_rate, side_rate] = match quality.rate {
                Rate::Fixed(log2) => [log2, log2],
                Rate::Auto => [
                    self.needs[0].next(mid, self.heard.inner().reach()),
                    // A corner that moves hard and fast shapes the
                    // sides as a curve driven that far would.
                    self.needs[1].next(side, (1.0 - self.open) * 4.0),
                ],
            };
            let bent_mid = self
                .heard
                .next(mid, 0.0, mid_rate, |core, x, _| core.next(x));
            let open = self.open;
            let bent_side = self
                .choke
                .next(side, open, side_rate, |choke, x, open| choke.next(x, open));

            // Both come out late, so what they are mixed with waits.
            let [left_was, right_was] = &mut self.waiting;
            left_was.write(left);
            right_was.write(right);
            let (left, right) = (left_was.read(late), right_was.read(late));
            let (mid, side) = ((left + right) * 0.5, (left - right) * 0.5);
            let mid = mid + (bent_mid - mid) * mix;
            let side = side + (bent_side - side) * mix;

            // Matched to what came in before the output trim, which is
            // then the person's own to set on top.
            let (out_l, out_r) = (mid + side, mid - side);
            let came = left * left + right * right;
            let made = out_l * out_l + out_r * out_r;
            let output = output * self.matcher.next(came, made, matching);
            // Soloing and weighing the halves come last.
            let (mid_gain, side_gain) = self.halves.next(balance, listen);
            let (mid, side) = (mid * mid_gain, side * side_gain);
            let outs = [(mid + side) * output, (mid - side) * output];
            for (ch, value) in outs.iter().enumerate().take(block.outputs()) {
                block.output(ch)[i] = *value;
            }
        }
        let mut loudest = 0.0f32;
        for ch in 0..block.outputs() {
            loudest = loudest.max(peak(block.output(ch)));
        }
        block.meter(INPUT, to_fraction(heard));
        block.meter(LEVEL, to_fraction(loudest));
        block.meter(FULL, self.full.min(1.0) as f64);
    }
}

// Sample `i` of sidechain channel `ch`: silence if nothing is routed.
#[inline]
fn key_sample(block: &Block, ch: usize, i: usize) -> f32 {
    block.key(ch).map_or(0.0, |buf| buf[i])
}

// Sample `i` of input `ch`; a mono input answers for both sides.
fn sample(block: &Block, ch: usize, i: usize) -> f32 {
    let ch = ch.min(block.inputs().saturating_sub(1));
    block.input(ch).get(i).copied().unwrap_or(0.0)
}

kit_vst3::export!(Ironside);
kit_clap::export!(Ironside);
