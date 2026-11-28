//! One row of a plugin's spec: a setting the host can automate, or a
//! meter it can only read. Hosts and the editor deal in normalized
//! values, 0 to 1; `plain` is the value in the setting's own terms.

use crate::level::fraction_to_db;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Kind {
    /// One of a few named choices. A toggle is a list of Off and On.
    List(&'static [&'static str]),
    /// A number from `lo` to `hi` in steps of `step` (0 for any), shown
    /// with `decimals` and `unit`. A `skew` over 1 gives the low end
    /// more of the travel, as a frequency wants.
    Range {
        lo: f64,
        hi: f64,
        step: f64,
        decimals: usize,
        unit: &'static str,
        skew: f64,
    },
    /// A meter: a level along `level::to_fraction`. Read-only.
    Level,
    /// A meter: any fraction the plugin reports. Read-only.
    Fraction,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Param {
    /// The host's id: never reused or renumbered once a build has
    /// shipped, since hosts save automation by it.
    pub id: u32,
    /// What the editor, the scenes and the docs call it.
    pub key: &'static str,
    pub title: &'static str,
    pub short: &'static str,
    /// Where the editor puts it.
    pub group: &'static str,
    /// One line on what it does, for the editor and the manual.
    pub about: &'static str,
    pub kind: Kind,
    /// The default, in the setting's own terms.
    pub default: f64,
}

const OFF_ON: [&str; 2] = ["Off", "On"];

/// A number from `lo` to `hi`, any value between, shown to 2 places.
pub const fn range(
    id: u32,
    key: &'static str,
    title: &'static str,
    group: &'static str,
    (lo, hi, default): (f64, f64, f64),
    unit: &'static str,
) -> Param {
    let kind = Kind::Range {
        lo,
        hi,
        step: 0.0,
        decimals: 2,
        unit,
        skew: 1.0,
    };
    Param {
        id,
        key,
        title,
        short: title,
        group,
        about: "",
        kind,
        default,
    }
}

/// One of `choices`, starting at `default`.
pub const fn list(
    id: u32,
    key: &'static str,
    title: &'static str,
    group: &'static str,
    choices: &'static [&'static str],
    default: usize,
) -> Param {
    Param {
        id,
        key,
        title,
        short: title,
        group,
        about: "",
        kind: Kind::List(choices),
        default: default as f64,
    }
}

pub const fn toggle(
    id: u32,
    key: &'static str,
    title: &'static str,
    group: &'static str,
    on: bool,
) -> Param {
    list(id, key, title, group, &OFF_ON, on as usize)
}

/// The key of the setting that is the plugin's own bypass.
pub const BYPASS: &str = "bypass";

/// The plugin's own bypass: Off, On. While it is on the runner fades
/// to the sound coming in, and both formats tell the host that this is
/// the bypass, so the host's own button and the plugin's are one.
pub const fn bypass(id: u32) -> Param {
    toggle(id, BYPASS, "Bypass", "", false).about("Passes the sound through unchanged")
}

/// The key of the setting that blends the effect with the sound that
/// came in.
pub const MIX: &str = "mix";

/// The key of the setting that holds the level going out to the level
/// coming in.
pub const MATCH: &str = "match";

/// How much of what goes out is the effect, 0 to 100%: every effect
/// it makes sense for has one, with this key, so a person finds it in
/// the same place and by the same name. The plugin does the blending,
/// since only it knows what its dry signal is.
pub const fn mix(id: u32) -> Param {
    range(id, MIX, "Mix", "Output", (0.0, 100.0, 100.0), "%")
        .decimals(0)
        .about("How much of what goes out is the effect, against the sound that came in")
}

/// Level matching, Off or On: every effect it makes sense for has one,
/// with this key. On, the level going out is held to the level coming
/// in, so a change is judged by its tone. `kit_dsp::Match` does the
/// work in the plugin's `process`, before its output trim; the header
/// shows it beside the bypass.
pub const fn level_match(id: u32) -> Param {
    toggle(id, MATCH, "Match", "Output", false).about(
        "Holds the level going out to the level coming in, so a change is heard as tone, not as louder",
    )
}

// ---- settings every plugin of a kind shares, so they mean the same
// thing and sit in the same place everywhere

const fn share(id: u32, key: &'static str, title: &'static str, group: &'static str) -> Param {
    range(id, key, title, group, (0.0, 100.0, 0.0), "%").decimals(0)
}

/// Mid and side, for any effect that works on them apart: one control
/// that turns the other half down, and a solo for each. The work is
/// `kit_dsp::MidSide`.
pub const fn balance(id: u32) -> Param {
    range(
        id,
        "balance",
        "Balance",
        "Output",
        (-100.0, 100.0, 0.0),
        "%",
    )
    .decimals(0)
    .about("Toward the middle or the sides: the other is turned down, neither is turned up")
}

pub const fn listen(id: u32) -> Param {
    list(
        id,
        "listen",
        "Listen",
        "Output",
        &["Both", "Mid", "Side"],
        0,
    )
    .about("Hear both, the middle alone, or the sides alone")
}

/// For an effect with a sidechain (`Plugin::KEY_IN`): whether what it
/// follows is its own input or the sidechain. Off by default, since a
/// host hands an unrouted sidechain over as silence.
pub const fn key(id: u32) -> Param {
    toggle(id, "key", "Sidechain", "Input", false)
        .about("Follows the track routed to the sidechain in place of its own sound")
}

/// For a plugin that bends the sound (`docs/quality.md`): how many
/// times the sample rate the bend runs at while playing, and what
/// happens when the host renders to a file. `quality` reads the two.
pub const fn oversampling(id: u32) -> Param {
    list(
        id,
        "oversampling",
        "Oversampling",
        "Quality",
        &["Auto", "Off", "2x", "4x", "8x", "16x"],
        0,
    )
    .about("How many times the rate the bend runs at while playing. Auto spends it only where it would be heard")
}

pub const fn render_quality(id: u32) -> Param {
    list(
        id,
        "render",
        "When rendering",
        "Quality",
        &["Best", "As playing", "Off", "2x", "4x", "8x", "16x"],
        0,
    )
    .about("What a render to a file gets. Best is the highest rate and the finest of everything, whatever it costs")
}

/// The rate a bend should run at.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Rate {
    /// Whatever the sound needs.
    Auto,
    /// This many doublings, 0 for none.
    Fixed(usize),
}

/// What the two quality settings come to at this moment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Quality {
    pub rate: Rate,
    /// Rendering at Best: the finest of everything else as well.
    pub best: bool,
}

/// Reads `oversampling` and `render_quality`, as choices, for a host
/// that is `rendering` or not.
pub const fn quality(playing: usize, render: usize, rendering: bool) -> Quality {
    let played = match playing {
        0 => Rate::Auto,
        n => Rate::Fixed(n - 1),
    };
    let rate = match (rendering, render) {
        (false, _) | (true, 1) => played,
        (true, 0) => Rate::Fixed(4),
        (true, n) => Rate::Fixed(n - 2),
    };
    Quality {
        rate,
        best: rendering && render == 0,
    }
}

/// The valve's own settings, for any effect that models one: they
/// live in the tube window, and `kit_dsp::Tube` is the valve. Each
/// rests where the valve is a plain curve.
pub const fn tube_stage(id: u32) -> Param {
    list(
        id,
        "tube_stage",
        "Stage",
        "Tube",
        &["Triode", "Strapped", "Pentode"],
        2,
    )
    .about("How the valve is wired: a triode adds even harmonics, a pentode odd, strapped between")
}

pub const fn tube_bias(id: u32) -> Param {
    range(id, "tube_bias", "Bias", "Tube", (-100.0, 100.0, 0.0), "%")
        .decimals(0)
        .about("Where on its curve the valve rests: off centre adds even harmonics")
}

pub const fn tube_grid(id: u32) -> Param {
    share(id, "tube_grid", "Grid", "Tube")
        .about("How suddenly the top half of a wave gives out, as a grid starts to conduct")
}

pub const fn tube_bloom(id: u32) -> Param {
    share(id, "tube_bloom", "Bloom", "Tube")
        .about("How far a loud sound pushes the valve off its rest, and how slowly it comes back")
}

pub const fn tube_feedback(id: u32) -> Param {
    share(id, "tube_feedback", "Feedback", "Tube")
        .about("How much the stage holds itself straight: cleaner, then a harder knee")
}

pub const fn tube_sag(id: u32) -> Param {
    share(id, "tube_sag", "Sag", "Tube")
        .about("How far a held loud sound eases back as the supply droops")
}

pub const fn tube_glow(id: u32) -> Param {
    share(id, "tube_glow", "Glow", "Tube")
        .about("How high the harmonics it adds sit: the sheen of the top end")
}

/// The transformer's own settings, for any effect that models one:
/// they live in the transformer window, and `kit_dsp::Transformer` is
/// the iron. Each rests where the transformer does nothing.
pub const fn iron_drive(id: u32) -> Param {
    share(id, "iron_drive", "Drive", "Transformer").about(
        "How hard the sound is pushed into the core: the lower the note, the sooner it fills",
    )
}

pub const fn iron_size(id: u32) -> Param {
    range(
        id,
        "iron_size",
        "Size",
        "Transformer",
        (20.0, 1000.0, 60.0),
        "Hz",
    )
    .skew(2.5)
    .decimals(0)
    .about("How much iron: the note at which a full level fills the core. Lower is bigger")
}

pub const fn iron_material(id: u32) -> Param {
    share(id, "iron_material", "Iron", "Transformer").about(
        "From nickel, clean until a sudden knee, to steel, which lags and is heard when quiet",
    )
}

pub const fn iron_source(id: u32) -> Param {
    share(id, "iron_source", "Source", "Transformer").about(
        "How much what feeds it resists: more of the core is heard, and less bass until loud",
    )
}

pub const fn iron_grip(id: u32) -> Param {
    share(id, "iron_grip", "Grip", "Transformer")
        .about("A standing current in a gapped core: even harmonics, and some bass given up")
}

pub const fn iron_top(id: u32) -> Param {
    share(id, "iron_top", "Top", "Transformer")
        .about("How far down into hearing the windings' resonance comes")
}

pub const fn iron_ring(id: u32) -> Param {
    range(
        id,
        "iron_ring",
        "Ring",
        "Transformer",
        (0.0, 100.0, 20.0),
        "%",
    )
    .decimals(0)
    .about("How little the load damps that resonance: a smooth roll, or a peak with an edge")
}

/// A level meter.
pub const fn meter(id: u32, key: &'static str, title: &'static str) -> Param {
    Param {
        id,
        key,
        title,
        short: title,
        group: "",
        about: "",
        kind: Kind::Level,
        default: 0.0,
    }
}

impl Param {
    pub const fn short(mut self, short: &'static str) -> Self {
        self.short = short;
        self
    }

    pub const fn about(mut self, about: &'static str) -> Self {
        self.about = about;
        self
    }

    /// For a range: values land on multiples of `step` above `lo`.
    pub const fn step(mut self, by: f64) -> Self {
        if let Kind::Range {
            lo,
            hi,
            decimals,
            unit,
            skew,
            ..
        } = self.kind
        {
            self.kind = Kind::Range {
                lo,
                hi,
                step: by,
                decimals,
                unit,
                skew,
            };
        }
        self
    }

    pub const fn decimals(mut self, places: usize) -> Self {
        if let Kind::Range {
            lo,
            hi,
            step,
            unit,
            skew,
            ..
        } = self.kind
        {
            self.kind = Kind::Range {
                lo,
                hi,
                step,
                decimals: places,
                unit,
                skew,
            };
        }
        self
    }

    pub const fn skew(mut self, by: f64) -> Self {
        if let Kind::Range {
            lo,
            hi,
            step,
            decimals,
            unit,
            ..
        } = self.kind
        {
            self.kind = Kind::Range {
                lo,
                hi,
                step,
                decimals,
                unit,
                skew: by,
            };
        }
        self
    }

    /// A meter of any fraction rather than a level.
    pub const fn fraction(mut self) -> Self {
        self.kind = Kind::Fraction;
        self
    }

    /// VST3 step count: list length less one, 0 for a continuous value.
    pub fn steps(&self) -> i32 {
        match self.kind {
            Kind::List(names) => names.len() as i32 - 1,
            Kind::Range { lo, hi, step, .. } if step > 0.0 => ((hi - lo) / step).round() as i32,
            Kind::Range { .. } | Kind::Level | Kind::Fraction => 0,
        }
    }

    /// Whether this is the plugin's own bypass.
    pub fn is_bypass(&self) -> bool {
        self.key == BYPASS && self.is_list()
    }

    pub fn is_list(&self) -> bool {
        matches!(self.kind, Kind::List(_))
    }

    pub fn read_only(&self) -> bool {
        matches!(self.kind, Kind::Level | Kind::Fraction)
    }

    pub fn index(&self, norm: f64) -> usize {
        let steps = self.steps().max(1) as f64;
        (norm.clamp(0.0, 1.0) * steps).round() as usize
    }

    /// The value in its own terms: a list's index, a range's number.
    pub fn plain(&self, norm: f64) -> f64 {
        let norm = norm.clamp(0.0, 1.0);

        match self.kind {
            Kind::List(_) => self.index(norm) as f64,
            Kind::Range {
                lo, hi, step, skew, ..
            } => {
                let value = lo + norm.powf(skew) * (hi - lo);
                match step > 0.0 {
                    true => (lo + ((value - lo) / step).round() * step).min(hi),
                    false => value,
                }
            }
            Kind::Level | Kind::Fraction => norm,
        }
    }

    pub fn normalized(&self, plain: f64) -> f64 {
        let norm = match self.kind {
            Kind::List(_) => plain / self.steps().max(1) as f64,
            Kind::Range { lo, hi, skew, .. } => {
                ((plain - lo) / (hi - lo)).clamp(0.0, 1.0).powf(1.0 / skew)
            }
            Kind::Level | Kind::Fraction => plain,
        };
        norm.clamp(0.0, 1.0)
    }

    /// The default as hosts hold it.
    pub fn default_norm(&self) -> f64 {
        self.normalized(self.default)
    }

    pub fn display(&self, norm: f64) -> String {
        match self.kind {
            Kind::List(names) => names[self.index(norm).min(names.len() - 1)].to_string(),
            Kind::Range {
                decimals, unit: "", ..
            } => format!("{:.*}", decimals, self.plain(norm)),
            Kind::Range { decimals, unit, .. } => {
                format!("{:.*} {unit}", decimals, self.plain(norm))
            }
            Kind::Level if norm <= 0.0 => "silent".to_string(),
            Kind::Level => format!("{:.0} dB", fraction_to_db(norm)),
            Kind::Fraction => format!("{:.0}%", norm.clamp(0.0, 1.0) * 100.0),
        }
    }

    /// The normalized value for text a person typed, or `None`.
    pub fn parse(&self, text: &str) -> Option<f64> {
        let text = text.trim();

        match self.kind {
            Kind::List(names) => names
                .iter()
                .position(|n| n.eq_ignore_ascii_case(text))
                .map(|i| i as f64 / (names.len() - 1).max(1) as f64),
            Kind::Range { lo, hi, unit, .. } => text
                .strip_suffix(unit)
                .unwrap_or(text)
                .trim()
                .parse::<f64>()
                .ok()
                .filter(|v| (lo..=hi).contains(v))
                .map(|v| self.normalized(v)),
            Kind::Level | Kind::Fraction => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const CUTOFF: Param = range(
        0,
        "cutoff",
        "Cutoff",
        "Filter",
        (20.0, 20_000.0, 1000.0),
        "Hz",
    )
    .skew(3.0)
    .decimals(0);
    const TIME: Param = range(1, "time", "Time", "Delay", (0.0, 500.0, 120.0), "ms")
        .step(5.0)
        .decimals(0);
    const MIX: Param = range(2, "mix", "Mix", "Out", (0.0, 1.0, 0.5), "");
    const MODE: Param = list(3, "mode", "Mode", "Out", &["Soft", "Hard", "Fold"], 1);
    const BYPASS: Param = toggle(4, "bypass", "Bypass", "Out", false);

    #[test]
    fn plain_and_normalized_round_trip() {
        let cases = [
            (CUTOFF, 1000.0),
            (CUTOFF, 20.0),
            (CUTOFF, 20_000.0),
            (TIME, 120.0),
            (MIX, 0.25),
            (MODE, 2.0),
            (BYPASS, 1.0),
        ];

        for (p, plain) in cases {
            let back = p.plain(p.normalized(plain));
            assert!(
                (back - plain).abs() < 1e-6,
                "{}: {plain} came back {back}",
                p.key
            );
        }
    }

    #[test]
    fn a_skew_gives_the_low_end_more_travel() {
        assert!(CUTOFF.plain(0.5) < 3000.0, "{}", CUTOFF.plain(0.5));
        assert!((CUTOFF.default_norm() - CUTOFF.normalized(1000.0)).abs() < 1e-12);
    }

    #[test]
    fn steps_are_what_a_host_counts() {
        let cases = [(CUTOFF, 0), (TIME, 100), (MIX, 0), (MODE, 2), (BYPASS, 1)];

        for (p, want) in cases {
            assert_eq!(p.steps(), want, "{}", p.key);
        }
        assert_eq!(TIME.plain(0.123), 60.0, "lands on a step");
    }

    #[test]
    fn shows_and_reads_its_own_text() {
        let cases = [
            (CUTOFF, 1000.0, "1000 Hz"),
            (TIME, 60.0, "60 ms"),
            (MIX, 0.25, "0.25"),
            (MODE, 2.0, "Fold"),
            (BYPASS, 0.0, "Off"),
        ];

        for (p, plain, text) in cases {
            let norm = p.normalized(plain);
            assert_eq!(p.display(norm), text, "{}", p.key);
            let read = p.parse(text).map(|n| p.plain(n));
            assert!(
                read.is_some_and(|v| (v - plain).abs() < 1e-6),
                "{text}: {read:?}"
            );
        }
        assert_eq!(MODE.parse("fold"), Some(1.0), "any case");
        assert_eq!(TIME.parse("900 ms"), None, "out of range");
        assert_eq!(MIX.parse("loud"), None);
    }

    #[test]
    fn meters_are_read_only_and_say_decibels() {
        let level = meter(9, "out", "Output");

        assert!(level.read_only() && !MIX.read_only());
        assert_eq!(level.display(0.0), "silent");
        assert_eq!(level.display(0.5), "-30 dB");
        assert_eq!(level.fraction().display(0.5), "50%");
        assert_eq!(level.parse("-30 dB"), None);
    }
}
