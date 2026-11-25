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
