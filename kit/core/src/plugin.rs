//! The one trait a plugin implements, and the spec the format crates
//! and the editor read from it.

use crate::Param;
use crate::runner::Block;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    /// Audio in, audio out.
    Effect,
    /// Notes in, audio out.
    Instrument,
}

/// What kind of thing a plugin is, which is where a host files it:
/// without one, Cubase puts every plugin under "Other". The first
/// group is for effects and the second for instruments.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Category {
    /// Not said. The suite every plugin runs refuses this.
    Other,
    Distortion,
    Dynamics,
    Eq,
    Filter,
    Delay,
    Reverb,
    Modulation,
    Spatial,
    PitchShift,
    Mastering,
    Restoration,
    Analyzer,
    Tools,
    Synth,
    Sampler,
    Drum,
    Piano,
}

impl Category {
    /// Whether this is a kind of instrument, not of effect.
    pub const fn plays(self) -> bool {
        matches!(self, Self::Synth | Self::Sampler | Self::Drum | Self::Piano)
    }

    /// The VST3 subcategories: Steinberg's own strings, which Cubase
    /// and Nuendo sort their plugin menus by.
    pub const fn vst3(self, role: Role) -> &'static str {
        match self {
            Self::Distortion => "Fx|Distortion",
            Self::Dynamics => "Fx|Dynamics",
            Self::Eq => "Fx|EQ",
            Self::Filter => "Fx|Filter",
            Self::Delay => "Fx|Delay",
            Self::Reverb => "Fx|Reverb",
            Self::Modulation => "Fx|Modulation",
            Self::Spatial => "Fx|Spatial",
            Self::PitchShift => "Fx|Pitch Shift",
            Self::Mastering => "Fx|Mastering",
            Self::Restoration => "Fx|Restoration",
            Self::Analyzer => "Fx|Analyzer",
            Self::Tools => "Fx|Tools",
            Self::Synth => "Instrument|Synth",
            Self::Sampler => "Instrument|Sampler",
            Self::Drum => "Instrument|Drum",
            Self::Piano => "Instrument|Piano",
            Self::Other => match role {
                Role::Instrument => "Instrument",
                Role::Effect => "Fx",
            },
        }
    }

    /// The CLAP feature that says the same, where CLAP has one.
    pub const fn clap(self) -> Option<&'static str> {
        Some(match self {
            Self::Distortion => "distortion",
            Self::Dynamics => "compressor",
            Self::Eq => "equalizer",
            Self::Filter => "filter",
            Self::Delay => "delay",
            Self::Reverb => "reverb",
            Self::Modulation => "chorus",
            Self::Spatial => "stereo",
            Self::PitchShift => "pitch-shifter",
            Self::Mastering => "mastering",
            Self::Restoration => "restoration",
            Self::Analyzer => "analyzer",
            Self::Tools => "utility",
            Self::Synth => "synthesizer",
            Self::Sampler => "sampler",
            Self::Drum => "drum",
            Self::Piano => "synthesizer",
            Self::Other => return None,
        })
    }
}

/// Who the plugin is. Everything a host or an installer shows comes
/// from here.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Info {
    pub name: &'static str,
    pub vendor: &'static str,
    pub url: &'static str,
    /// One line, as a host's plugin list shows it.
    pub description: &'static str,
    /// `env!("CARGO_PKG_VERSION")` of the plugin's crate.
    pub version: &'static str,
    /// The CLAP id, reverse-domain. Fixed for good.
    pub clap_id: &'static str,
    /// The VST3 class ids of the processor and the controller. Fixed
    /// for good: hosts save projects by them.
    pub vst3_processor: [u32; 4],
    pub vst3_controller: [u32; 4],
    pub role: Role,
    /// Channels of the main audio input (0 for none) and output.
    pub audio_in: u32,
    pub audio_out: u32,
    /// Whether it takes notes in and sends MIDI out.
    pub midi_in: bool,
    pub midi_out: bool,
    /// The editor's logical size, and the smallest it may be made.
    pub size: (u32, u32),
    pub min_size: (u32, u32),
}

/// A file of the editor's page: its path from the root, its MIME type
/// and its text.
pub type Asset = (&'static str, &'static str, &'static str);

/// An audio plugin. `new` and `prepare` may allocate; `process` runs
/// on the audio thread and must not allocate, lock or wait.
pub trait Plugin: Send + 'static {
    const INFO: &'static Info;
    /// The settings, in the order hosts list them. A row's index here
    /// is how `Block` reads it; its `id` is how hosts know it.
    const PARAMS: &'static [Param];
    /// What it reports back, each block: indexes for `Block::meter`.
    const METERS: &'static [Param] = &[];
    /// The editor's own files, beside the kit's (`main.js`, at least).
    const ASSETS: &'static [Asset] = &[];
    /// Channels of a sidechain: a second input the host can route
    /// another track to, read with `Block::key`. 0 for none. Only an
    /// effect has one.
    const KEY_IN: u32 = 0;
    /// What kind of thing it is, for a host's menus. Every plugin in
    /// `plugins/` says: the suite fails one that leaves it at `Other`
    /// or names an instrument's kind for an effect.
    const CATEGORY: Category = Category::Other;

    fn new() -> Self;

    /// Called off the audio thread before any processing, and again
    /// whenever the rate or the largest block changes.
    fn prepare(&mut self, rate: f32, max_block: usize);

    /// Whether the host is rendering to a file, not playing. Called
    /// off the audio thread, and `prepare` is called again after it,
    /// so a plugin may be later, and cost more, when rendering.
    fn rendering(&mut self, _on: bool) {}
    /// Forget what has been heard: the transport jumped or stopped.
    fn reset(&mut self) {}

    /// Samples the output runs behind the input, as last prepared.
    fn latency(&self) -> u32 {
        0
    }

    /// Samples it keeps sounding after the input stops.
    fn tail(&self) -> u32 {
        0
    }

    fn process(&mut self, block: &mut Block);
}

/// A plugin's spec without its type, for code that serves any plugin.
#[derive(Debug, Clone, Copy)]
pub struct Spec {
    pub info: &'static Info,
    pub params: &'static [Param],
    pub meters: &'static [Param],
    pub assets: &'static [Asset],
    /// Channels of the sidechain input, 0 for none.
    pub key_in: u32,
    /// What kind of thing it is, for a host's menus.
    pub category: Category,
}

impl Spec {
    pub const fn of<P: Plugin>() -> Self {
        Self {
            info: P::INFO,
            params: P::PARAMS,
            meters: P::METERS,
            assets: P::ASSETS,
            key_in: match P::INFO.audio_in {
                0 => 0,
                _ => P::KEY_IN,
            },
            category: P::CATEGORY,
        }
    }

    /// Settings and meters together.
    pub fn total(&self) -> usize {
        self.params.len() + self.meters.len()
    }

    /// The setting or meter at `index`: settings first, then meters.
    pub fn row(&self, index: usize) -> Option<&'static Param> {
        self.params
            .get(index)
            .or_else(|| self.meters.get(index.checked_sub(self.params.len())?))
    }

    pub fn rows(&self) -> impl Iterator<Item = &'static Param> {
        self.params.iter().chain(self.meters)
    }

    /// The index of the setting or meter a host knows as `id`.
    pub fn index_of(&self, id: u32) -> Option<usize> {
        self.rows().position(|p| p.id == id)
    }

    /// The index of the setting alone a host knows as `id`.
    pub fn setting_of(&self, id: u32) -> Option<usize> {
        self.params.iter().position(|p| p.id == id)
    }

    pub fn by_id(&self, id: u32) -> Option<&'static Param> {
        self.rows().find(|p| p.id == id)
    }

    pub fn by_key(&self, key: &str) -> Option<&'static Param> {
        self.rows().find(|p| p.key == key)
    }

    /// Every setting at its default, normalized.
    pub fn defaults(&self) -> Vec<f64> {
        self.params.iter().map(Param::default_norm).collect()
    }

    /// What is wrong with the spec, if anything: a host would trip on
    /// each of these, or a later build would break saved projects.
    pub fn problems(&self) -> Vec<String> {
        let mut out = Vec::new();
        let rows: Vec<_> = self.rows().collect();
        for (i, p) in rows.iter().enumerate() {
            if rows[..i].iter().any(|q| q.id == p.id) {
                out.push(format!("{}: id {} is used twice", p.key, p.id));
            }
            if rows[..i].iter().any(|q| q.key == p.key) {
                out.push(format!("{}: key is used twice", p.key));
            }
            if p.key.is_empty() || p.title.is_empty() {
                out.push(format!("id {}: needs a key and a title", p.id));
            }
            let back = p.plain(p.default_norm());
            if !p.read_only() && (back - p.default).abs() > 1e-6 {
                out.push(format!(
                    "{}: default {} is not a value it can hold",
                    p.key, p.default
                ));
            }
        }
        for p in self.params.iter().filter(|p| p.read_only()) {
            out.push(format!("{}: a meter among the settings", p.key));
        }
        for p in self.meters.iter().filter(|p| !p.read_only()) {
            out.push(format!("{}: a setting among the meters", p.key));
        }
        let info = self.info;
        if info.vst3_processor == info.vst3_controller {
            out.push("the VST3 processor and controller share a class id".into());
        }
        if !info.clap_id.contains('.') || info.clap_id.contains(' ') {
            out.push(format!("CLAP id {:?} is not reverse-domain", info.clap_id));
        }
        if info.audio_out == 0 {
            out.push("no audio output".into());
        }
        if info.role == Role::Instrument && !info.midi_in {
            out.push("an instrument that takes no notes".into());
        }
        out
    }
}
