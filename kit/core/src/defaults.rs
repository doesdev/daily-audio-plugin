//! Choices a person wants every new instance to start from
//! (`docs/quality.md`): kept in small files beside the presets, one
//! for each plugin and one for every DAP plugin, read when an
//! instance is made. A project that was saved keeps what it was saved
//! with; these are only where a fresh instance starts.
//!
//! A file is lines of a setting's key, a tab, and its value in its
//! own units. The plugin's own file is read after the one for all, so
//! it wins, and saying a default for all takes that key out of every
//! plugin's file, so the last thing said is what holds.

use crate::{Shared, Spec};
use std::path::{Path, PathBuf};

/// The file for every DAP plugin. A plugin's own is its name.
const ALL: &str = "Every plugin";

/// Where defaults are kept on this machine, if anywhere can be found.
pub fn dir() -> Option<PathBuf> {
    let home = || std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config"));
    let base = std::env::var_os("APPDATA")
        .or_else(|| std::env::var_os("XDG_CONFIG_HOME"))
        .map(PathBuf::from)
        .or_else(home)?;
    Some(base.join(crate::HOME).join("Defaults"))
}

fn file(dir: &Path, plugin: Option<&str>) -> PathBuf {
    dir.join(format!("{}.txt", plugin.unwrap_or(ALL)))
}

fn lines(path: &Path) -> Vec<(String, f64)> {
    let text = std::fs::read_to_string(path).unwrap_or_default();
    text.lines()
        .filter_map(|line| {
            let (key, plain) = line.split_once('\t')?;
            Some((key.to_string(), plain.trim().parse().ok()?))
        })
        .collect()
}

// Written beside the file and moved over it, so a reader never finds
// half a file.
fn write(path: &Path, lines: &[(String, f64)]) -> std::io::Result<()> {
    if lines.is_empty() {
        return match std::fs::remove_file(path) {
            Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(e),
            _ => Ok(()),
        };
    }
    let text: String = lines.iter().map(|(k, v)| format!("{k}\t{v}\n")).collect();
    let beside = path.with_extension("new");
    std::fs::write(&beside, text)?;
    std::fs::rename(&beside, path)
}

/// Keeps `plain` as where setting `key` starts: for the plugin called
/// `plugin`, or with `None` for every DAP plugin.
pub fn keep(dir: &Path, plugin: Option<&str>, key: &str, plain: f64) -> std::io::Result<()> {
    std::fs::create_dir_all(dir)?;
    if plugin.is_none() {
        // The last thing said holds: no plugin's own file may go on
        // saying otherwise.
        for entry in std::fs::read_dir(dir)?.flatten() {
            let path = entry.path();
            let own = path.extension().is_some_and(|e| e == "txt") && path != file(dir, None);
            let had = lines(&path);
            if own && had.iter().any(|(k, _)| k == key) {
                let left: Vec<_> = had.into_iter().filter(|(k, _)| k != key).collect();
                write(&path, &left)?;
            }
        }
    }
    let path = file(dir, plugin);
    let mut kept: Vec<_> = lines(&path).into_iter().filter(|(k, _)| k != key).collect();
    kept.push((key.to_string(), plain));
    write(&path, &kept)
}

/// Where each setting of `spec` should start, for those a default has
/// been kept for: its index and its value, normalized.
pub fn read(dir: &Path, spec: &Spec) -> Vec<(usize, f64)> {
    let all = lines(&file(dir, None));
    let own = lines(&file(dir, Some(spec.info.name)));
    let mut out: Vec<(usize, f64)> = Vec::new();
    for (key, plain) in all.into_iter().chain(own) {
        let found = spec
            .params
            .iter()
            .position(|p| p.key == key && !p.is_bypass() && !p.read_only());
        if let Some(index) = found {
            let norm = spec.params[index].normalized(plain);
            out.retain(|(i, _)| *i != index);
            out.push((index, norm));
        }
    }
    out
}

/// Starts a new instance's settings from what has been kept in `dir`.
pub fn apply(dir: &Path, shared: &Shared, spec: &Spec) {
    for (index, norm) in read(dir, spec) {
        shared.set(index, norm);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Info, Param, Role, bypass, oversampling, render_quality};

    const fn info(name: &'static str) -> Info {
        Info {
            name,
            vendor: "kit",
            url: "",
            description: "",
            version: "0",
            clap_id: "dev.kit.defaults",
            vst3_processor: [1, 2, 3, 4],
            vst3_controller: [5, 6, 7, 8],
            role: Role::Effect,
            audio_in: 2,
            audio_out: 2,
            midi_in: false,
            midi_out: false,
            size: (1, 1),
            min_size: (1, 1),
        }
    }

    const PARAMS: &[Param] = &[bypass(1), oversampling(2), render_quality(3)];

    fn spec(info: &'static Info) -> Spec {
        Spec {
            info,
            params: PARAMS,
            meters: &[],
            assets: &[],
            key_in: 0,
            category: crate::Category::Tools,
        }
    }

    fn fresh(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("dap-defaults-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    // The choice a new instance of `spec` would start at.
    fn starts(dir: &Path, spec: &Spec, index: usize) -> usize {
        let shared = Shared::new(spec);
        apply(dir, &shared, spec);
        spec.params[index].index(shared.value(index).unwrap())
    }

    #[test]
    fn a_new_instance_starts_from_what_was_kept_and_the_last_said_holds() {
        static ONE: Info = info("One");
        static TWO: Info = info("Two");
        let (one, two) = (spec(&ONE), spec(&TWO));
        let dir = fresh("holds");

        // Nothing kept: what the kit ships.
        assert_eq!((starts(&dir, &one, 1), starts(&dir, &two, 1)), (0, 0));

        // For one plugin: that plugin alone.
        keep(&dir, Some("One"), "oversampling", 3.0).unwrap();
        assert_eq!((starts(&dir, &one, 1), starts(&dir, &two, 1)), (3, 0));

        // For all: the others take it, the plugin's own still wins
        // where it spoke first on another setting.
        keep(&dir, Some("One"), "render", 2.0).unwrap();
        keep(&dir, None, "oversampling", 1.0).unwrap();
        assert_eq!((starts(&dir, &one, 1), starts(&dir, &two, 1)), (1, 1));
        assert_eq!((starts(&dir, &one, 2), starts(&dir, &two, 2)), (2, 0));

        // And the plugin can speak again after.
        keep(&dir, Some("One"), "oversampling", 5.0).unwrap();
        assert_eq!((starts(&dir, &one, 1), starts(&dir, &two, 1)), (5, 1));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn what_cannot_be_a_default_is_left_alone() {
        static ONE: Info = info("One");
        let one = spec(&ONE);
        let dir = fresh("alone");
        keep(&dir, None, "bypass", 1.0).unwrap();
        keep(&dir, None, "no such setting", 1.0).unwrap();
        std::fs::write(dir.join("One.txt"), "not a line\noversampling\tlots\n").unwrap();
        assert!(read(&dir, &one).is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
