//! Hand-Me-Down: each note keeps its own hit, and then plays the body
//! an earlier note left behind, from the same point in that note's
//! life. The pluck is now; the ring is one note (or up to four) late.
//!
//! Notes are found by a jump in level over the sound's own recent
//! level. Everything that comes in is kept in a ring a few seconds
//! long, with where each note started, so an earlier note's body is
//! read straight out of it: no copy is made and nothing is allocated.

use kit::level::{db_to_gain, peak, to_fraction};
use kit::{Asset, Block, Info, Param, Plugin, Role, list, meter, range};
use kit_dsp::{Follower, Match, MidSide, Smooth};

// Indexes into PARAMS, for `Block`.
pub const THRESHOLD: usize = 0;
pub const GAP: usize = 1;
pub const HIT: usize = 2;
pub const FADE: usize = 3;
pub const BACK: usize = 4;
pub const INTO: usize = 5;
pub const HIT_GAIN: usize = 6;
pub const BODY_GAIN: usize = 7;
pub const MIX: usize = 8;
pub const OUTPUT: usize = 9;
pub const BYPASS: usize = 10;
pub const MATCH: usize = 11;
pub const BALANCE: usize = 12;
pub const LISTEN: usize = 13;
pub const KEY: usize = 14;
// Indexes into METERS.
pub const INPUT: usize = 0;
pub const LEVEL: usize = 1;
pub const AGE: usize = 2;
pub const HEIR: usize = 3;

// Into: which half takes the body handed down.
pub const BOTH: usize = 0;
pub const MID: usize = 1;
pub const SIDE: usize = 2;

// How fast a moved level is followed, in ms.
const EASE_MS: f32 = 20.0;

/// How far back the sound is kept, in seconds: an earlier note's body
/// can be read for as long as it started no longer ago than this.
pub const MEMORY_S: f32 = 8.0;

/// How long the age meter takes to climb from a note's start to full,
/// in seconds: the editor finds notes where it drops.
pub const AGE_S: f32 = 4.0;

// A new hit fades in, and the body it replaces fades out, over this
// long: short enough to be heard as the hit's own start, long enough
// not to click.
const CROSS_MS: f32 = 1.0;

// A body handed down fades out over this long before the note it came
// from ended, so its own next hit is never heard.
const END_MS: f32 = 10.0;

// Below this a jump is not a note: about -50 dB.
const FLOOR: f32 = 0.003;

// How many note starts are remembered: the current one and four back,
// with room to spare.
const STARTS: usize = 8;

/// The sustain an earlier note left, as read now: how far back in the
/// ring it is, and where that note ended.
#[derive(Debug, Clone, Copy, Default)]
struct Reader {
    back: u64,
    end: u64,
    live: bool,
}

pub struct HandMeDown {
    rate: f32,
    // Everything that came in, a ring a side.
    ring: [Vec<f32>; 2],
    now: u64,
    starts: [u64; STARTS],
    notes: u64,
    since: u64,
    fast: Follower,
    slow: Follower,
    // The body being handed down to this note, and the one it took
    // over from, fading out.
    reader: Reader,
    leaving: Reader,
    leaving_from: f32,
    heir: f32,
    hit_gain: Smooth,
    body_gain: Smooth,
    mix: Smooth,
    matcher: Match,
    halves: MidSide,
    output: Smooth,
}

/// How much of the note's own sound is its hit, `i` samples after it
/// started: rising over the crossfade, held for `hit`, then fading
/// over `fade`, all in samples.
pub fn hit_weight(i: f32, cross: f32, hit: f32, fade: f32) -> f32 {
    let rise = (i / cross.max(1.0)).min(1.0);
    let fall = match i < hit {
        true => 1.0,
        false => (1.0 - (i - hit) / fade.max(1.0)).max(0.0),
    };
    rise * fall
}

impl Plugin for HandMeDown {
    // It shapes a sound's hit and body: where a host files a
    // transient shaper.
    const CATEGORY: kit::Category = kit::Category::Dynamics;
    const INFO: &'static Info = &Info {
        name: "Hand-Me-Down",
        vendor: kit::VENDOR,
        url: "https://github.com/doesdev/daily-audio-plugin",
        description: "Each note's hit, with the body an earlier note left",
        version: env!("CARGO_PKG_VERSION"),
        // Fixed for good: hosts save projects by these three.
        clap_id: "dev.doesdev.hand-me-down",
        vst3_processor: [0x3CE4_12EE, 0x6B7A_086F, 0xC6D3_12CB, 0xEA83_47E8],
        vst3_controller: [0x8C2B_F5BC, 0x2CF1_6207, 0x8B60_AAC9, 0x476E_E4E0],
        role: Role::Effect,
        audio_in: 2,
        audio_out: 2,
        midi_in: false,
        midi_out: false,
        size: (980, 540),
        min_size: (735, 405),
    };

    // The ids are fixed for good too. A new setting takes the next id
    // and goes wherever it reads best; none is ever reused.
    const PARAMS: &'static [Param] = &[
        range(0, "threshold", "Threshold", "Notes", (2.0, 24.0, 8.0), "dB")
            .decimals(1)
            .about("How far the sound must jump over its own recent level to count as a new note"),
        range(1, "gap", "Gap", "Notes", (20.0, 1000.0, 80.0), "ms")
            .skew(2.0)
            .step(1.0)
            .decimals(0)
            .about("The shortest time between two notes: a jump sooner belongs to the note already sounding"),
        range(2, "hit", "Hit", "Hit", (1.0, 250.0, 30.0), "ms")
            .skew(2.0)
            .step(1.0)
            .decimals(0)
            .about("How long each note keeps its own sound before the body handed down takes over"),
        range(3, "fade", "Fade", "Hit", (1.0, 200.0, 20.0), "ms")
            .skew(2.0)
            .step(1.0)
            .decimals(0)
            .about("How long the hit takes to hand over to the body"),
        list(
            4,
            "back",
            "Back",
            "Body",
            &["One", "Two", "Three", "Four"],
            0,
        )
        .about("How many notes back the body comes from"),
        list(
            5,
            "into",
            "Into",
            "Body",
            &["Both halves", "Middle", "Sides"],
            BOTH,
        )
            .about("Which half takes the body handed down: both, the middle alone, or the sides alone"),
        range(6, "hit_gain", "Hit level", "Hit", (-24.0, 12.0, 0.0), "dB")
            .decimals(1)
            .about("The level of each note's own hit"),
        range(7, "body_gain", "Body level", "Body", (-24.0, 12.0, 0.0), "dB")
            .decimals(1)
            .about("The level of the body that follows it"),
        kit::mix(8),
        range(9, "output", "Output", "Output", (-24.0, 12.0, 0.0), "dB")
            .decimals(1)
            .about("Level after everything else"),
        // Ids 10 to 13 are the meters'.
        kit::bypass(14),
        kit::level_match(15),
        kit::balance(16),
        kit::listen(17),
        kit::key(18),
    ];

    // A sidechain: another track can say where the notes start.
    const KEY_IN: u32 = 2;
    const METERS: &'static [Param] = &[
        meter(10, "input", "In"),
        meter(11, "level", "Out"),
        meter(12, "age", "Age").fraction(),
        meter(13, "heir", "Handed down").fraction(),
    ];

    const ASSETS: &'static [Asset] = &[
        ("/main.js", "text/javascript", include_str!("../ui/main.js")),
        ("/style.css", "text/css", include_str!("../ui/style.css")),
        (
            "/lineage.js",
            "text/javascript",
            include_str!("../ui/lineage.js"),
        ),
    ];

    fn new() -> Self {
        let mut plugin = Self {
            rate: 48_000.0,
            ring: [Vec::new(), Vec::new()],
            now: 0,
            starts: [0; STARTS],
            notes: 0,
            since: u64::MAX / 2,
            fast: Follower::new(0.1, 8.0, 48_000.0),
            slow: Follower::new(15.0, 150.0, 48_000.0),
            reader: Reader::default(),
            leaving: Reader::default(),
            leaving_from: 0.0,
            heir: 0.0,
            hit_gain: Smooth::new(1.0, EASE_MS, 48_000.0),
            body_gain: Smooth::new(1.0, EASE_MS, 48_000.0),
            mix: Smooth::new(1.0, EASE_MS, 48_000.0),
            matcher: Match::new(48_000.0),
            halves: MidSide::new(48_000.0),
            output: Smooth::new(1.0, EASE_MS, 48_000.0),
        };
        plugin.prepare(48_000.0, 1024);
        plugin
    }

    fn prepare(&mut self, rate: f32, _max_block: usize) {
        let at = |index: usize| Self::PARAMS[index].default as f32;
        self.rate = rate;
        let room = (MEMORY_S * rate) as usize + 1;
        self.ring = [vec![0.0; room], vec![0.0; room]];
        self.fast = Follower::new(0.1, 8.0, rate);
        self.slow = Follower::new(15.0, 150.0, rate);
        self.hit_gain = Smooth::new(db_to_gain(at(HIT_GAIN)), EASE_MS, rate);
        self.body_gain = Smooth::new(db_to_gain(at(BODY_GAIN)), EASE_MS, rate);
        self.mix = Smooth::new(at(MIX) / 100.0, EASE_MS, rate);
        self.matcher = Match::new(rate);
        self.halves = MidSide::new(rate);
        self.output = Smooth::new(db_to_gain(at(OUTPUT)), EASE_MS, rate);
        self.reset();
    }

    fn reset(&mut self) {
        self.ring.iter_mut().for_each(|r| r.fill(0.0));
        self.now = 0;
        self.starts = [0; STARTS];
        self.notes = 0;
        self.since = u64::MAX / 2;
        self.fast.reset();
        self.slow.reset();
        self.reader = Reader::default();
        self.leaving = Reader::default();
        self.leaving_from = 0.0;
        self.heir = 0.0;
    }

    fn process(&mut self, block: &mut Block) {
        let threshold = db_to_gain(block.value(THRESHOLD));
        let gap = (block.value(GAP) * 0.001 * self.rate) as u64;
        let hit = block.value(HIT) * 0.001 * self.rate;
        let fade = block.value(FADE) * 0.001 * self.rate;
        let back = block.choice(BACK) as u64 + 1;
        let into = block.choice(INTO);
        let hit_gain = db_to_gain(block.value(HIT_GAIN));
        let body_gain = db_to_gain(block.value(BODY_GAIN));
        let mix = block.value(MIX) / 100.0;
        let output = db_to_gain(block.value(OUTPUT));
        let matching = block.choice(MATCH) == 1;
        let balance = block.value(BALANCE) / 100.0;
        let listen = block.choice(LISTEN);
        let keyed = block.on(KEY);
        let cross = CROSS_MS * 0.001 * self.rate;
        let end_fade = END_MS * 0.001 * self.rate;
        let room = self.ring[0].len() as u64;

        let mut heard = 0.0f32;
        for ch in 0..block.inputs() {
            heard = heard.max(peak(block.input(ch)));
        }
        for i in 0..block.frames() {
            let (left, right) = (sample(block, 0, i), sample(block, 1, i));
            let slot = (self.now % room) as usize;
            self.ring[0][slot] = left;
            self.ring[1][slot] = right;

            // Where notes start: a jump over the sound's own recent
            // level, or with the sidechain on, the other track's.
            let level = match keyed {
                true => (key_sample(block, 0, i).abs() + key_sample(block, 1, i).abs()) * 0.5,
                false => (left.abs() + right.abs()) * 0.5,
            };
            let (fast, slow) = (self.fast.next(level), self.slow.next(level));
            self.since = self.since.saturating_add(1);
            if self.since >= gap && fast > FLOOR && fast > slow * threshold {
                self.start_note(back, room, end_fade);
            }

            // How far into its note this sample is, and how much of it
            // is the note's own hit.
            let age = self.since as f32;
            let own = match self.notes {
                0 => 0.0,
                _ => hit_weight(age, cross, hit, fade),
            };
            // The body handed down fades in under the new hit as the
            // one it replaces fades out, and fades out itself before
            // the note it came from ended.
            let rise = (age / cross).min(1.0);
            let heir = rise * self.reader.weight(self.now, end_fade);
            let leaving =
                (1.0 - rise) * self.leaving_from * self.leaving.weight(self.now, end_fade);
            self.heir = heir;
            let read = |r: &Reader, ring: &[f32]| {
                let at = self.now.saturating_sub(r.back);
                ring[(at % room) as usize]
            };
            let handed = |ring: &[f32], dry: f32| {
                heir * read(&self.reader, ring)
                    + leaving * read(&self.leaving, ring)
                    + (1.0 - heir - leaving) * dry
            };
            let (body_l, body_r) = (handed(&self.ring[0], left), handed(&self.ring[1], right));
            // Into the middle or the sides alone: the other half keeps
            // its own body.
            let (own_mid, own_side) = ((left + right) * 0.5, (left - right) * 0.5);
            let (got_mid, got_side) = ((body_l + body_r) * 0.5, (body_l - body_r) * 0.5);
            let (body_mid, body_side) = match into {
                MID => (got_mid, own_side),
                SIDE => (own_mid, got_side),
                _ => (got_mid, got_side),
            };

            let (hit_gain, body_gain) =
                (self.hit_gain.next(hit_gain), self.body_gain.next(body_gain));
            let (mix, output) = (self.mix.next(mix), self.output.next(output));
            let shaped = |own_half: f32, body: f32| {
                let wet = own * hit_gain * own_half + (1.0 - own) * body_gain * body;
                own_half + (wet - own_half) * mix
            };
            let (mid, side) = (shaped(own_mid, body_mid), shaped(own_side, body_side));

            // Matched to what came in before the output trim, which is
            // then the person's own to set on top.
            let (out_l, out_r) = (mid + side, mid - side);
            let heard_now = left * left + right * right;
            let made = out_l * out_l + out_r * out_r;
            let output = output * self.matcher.next(heard_now, made, matching);
            // Soloing and weighing the halves come last, after the
            // level is matched on the whole.
            let (mid_gain, side_gain) = self.halves.next(balance, listen);
            let (mid, side) = (mid * mid_gain, side * side_gain);
            let outs = [(mid + side) * output, (mid - side) * output];
            for (ch, value) in outs.iter().enumerate().take(block.outputs()) {
                block.output(ch)[i] = *value;
            }
            self.now += 1;
        }
        let mut loudest = 0.0f32;
        for ch in 0..block.outputs() {
            loudest = loudest.max(peak(block.output(ch)));
        }
        let age = match self.notes {
            0 => 1.0,
            _ => (self.since as f32 / (AGE_S * self.rate)).min(1.0),
        };
        block.meter(INPUT, to_fraction(heard));
        block.meter(LEVEL, to_fraction(loudest));
        block.meter(AGE, age as f64);
        block.meter(HEIR, self.heir.clamp(0.0, 1.0) as f64);
    }
}

impl HandMeDown {
    // A new note starts now. The body being handed down starts to fade
    // out, and the one `back` notes before this takes its place, if
    // the ring still holds where it started.
    fn start_note(&mut self, back: u64, room: u64, end_fade: f32) {
        let rise = (self.since as f32 / (CROSS_MS * 0.001 * self.rate)).min(1.0);
        let was = rise * self.reader.weight(self.now, end_fade);
        let fading = (1.0 - rise) * self.leaving_from * self.leaving.weight(self.now, end_fade);
        // Whichever was louder in the body now is the one let go of
        // gently; a cut that short is under the new hit either way.
        if was >= fading {
            self.leaving = self.reader;
            self.leaving_from = was;
        } else {
            self.leaving_from = fading;
        }
        self.starts[(self.notes % STARTS as u64) as usize] = self.now;
        self.notes += 1;
        self.since = 0;
        self.reader = match self.notes > back {
            false => Reader::default(),
            true => {
                let at = |k: u64| self.starts[((self.notes - 1 - k) % STARTS as u64) as usize];
                let (from, end) = (at(back), at(back - 1));
                Reader {
                    back: self.now - from,
                    end,
                    live: self.now - from < room - 1,
                }
            }
        };
    }
}

impl Reader {
    // How much of the body this reader gives at `now`: all of it until
    // the note it reads from is about to end, then less, then none.
    fn weight(&self, now: u64, end_fade: f32) -> f32 {
        if !self.live {
            return 0.0;
        }
        let at = now.saturating_sub(self.back);
        let left = self.end.saturating_sub(at) as f32;
        (left / end_fade.max(1.0)).min(1.0)
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

kit_vst3::export!(HandMeDown);
kit_clap::export!(HandMeDown);
