//! A person's own presets, as files: one JSON file a preset, in a
//! folder of the plugin's own, so they can be backed up, sent to
//! someone, and are still there after the host's cache is cleared.
//! The page asks for them to be saved and deleted; nothing here trusts
//! what it sends beyond a name and numbers from 0 to 1.

use kit::Spec;
use serde_json::{Map, Value, json};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// Where this plugin's presets are kept: under the roaming application
/// data on Windows, the configuration folder elsewhere.
pub fn dir(spec: &Spec) -> Option<PathBuf> {
    let home = || std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config"));
    let base = std::env::var_os("APPDATA")
        .or_else(|| std::env::var_os("XDG_CONFIG_HOME"))
        .map(PathBuf::from)
        .or_else(home)?;
    Some(
        // Not the vendor's name: see `kit::HOME`.
        base.join(kit::HOME).join(spec.info.name).join("Presets"),
    )
}

/// The file a preset called `name` is kept in: the name with anything
/// a file system might object to left out, or `None` if nothing is
/// left.
pub fn file_name(name: &str) -> Option<String> {
    let kept: String = name
        .chars()
        .filter(|c| c.is_alphanumeric() || " -_(),.'&+".contains(*c))
        .collect();
    let kept = kept.trim().trim_matches('.').trim();
    (!kept.is_empty()).then(|| format!("{kept}.json"))
}

/// The settings of a preset as the page may send them: by key, each a
/// number from 0 to 1. Anything else is left out.
fn settings(values: &Value) -> Map<String, Value> {
    values
        .as_object()
        .into_iter()
        .flatten()
        .filter_map(|(key, v)| {
            let norm = v.as_f64().filter(|n| n.is_finite())?.clamp(0.0, 1.0);
            Some((key.clone(), json!(norm)))
        })
        .collect()
}

/// Every preset in `dir`, by name. A file that is not one is skipped.
pub fn list(dir: &Path) -> Vec<Value> {
    let read = |path: PathBuf| {
        let preset: Value = serde_json::from_str(&fs::read_to_string(path).ok()?).ok()?;
        let name = preset.get("name")?.as_str()?.to_owned();
        Some(json!({ "name": name, "values": settings(preset.get("values")?) }))
    };
    let mut found: Vec<Value> = fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|e| e == "json"))
        .filter_map(read)
        .collect();
    found.sort_by_key(|preset| preset["name"].as_str().map(str::to_lowercase));
    found
}

pub fn save(spec: &Spec, dir: &Path, name: &str, values: &Value) -> io::Result<()> {
    let file = file_name(name).ok_or(io::ErrorKind::InvalidInput)?;
    let preset = json!({
        "plugin": spec.info.name,
        "version": spec.info.version,
        "name": name,
        "values": settings(values),
    });
    fs::create_dir_all(dir)?;
    fs::write(
        dir.join(file),
        serde_json::to_string_pretty(&preset).unwrap_or_default(),
    )
}

pub fn delete(dir: &Path, name: &str) -> io::Result<()> {
    let file = file_name(name).ok_or(io::ErrorKind::InvalidInput)?;
    fs::remove_file(dir.join(file))
}

/// The presets in `dir`, as the page is told of them.
pub fn message(dir: &Path) -> String {
    json!({
        "type": "presets",
        "list": list(dir),
        "dir": dir.to_string_lossy(),
    })
    .to_string()
}

/// Does what a preset message from the page asks and answers with the
/// presets as they now are; `None` if `text` is not a preset message.
pub fn handle(spec: &Spec, dir: &Path, text: &str) -> Option<String> {
    let msg: Value = serde_json::from_str(text).ok()?;
    let name = || msg.get("name").and_then(Value::as_str);
    // A failure leaves the files as they were, and the answer says so
    // by listing them unchanged.
    match msg.get("type")?.as_str()? {
        "preset-save" => drop(save(spec, dir, name()?, msg.get("values")?)),
        "preset-delete" => drop(delete(dir, name()?)),
        _ => return None,
    }
    Some(message(dir))
}

#[cfg(test)]
mod tests {
    use super::*;
    use kit::{Info, Role};

    const INFO: Info = Info {
        name: "Thing",
        vendor: "kit",
        url: "",
        description: "",
        version: "1.2.3",
        clap_id: "dev.kit.thing",
        vst3_processor: [0; 4],
        vst3_controller: [1; 4],
        role: Role::Effect,
        audio_in: 2,
        audio_out: 2,
        midi_in: false,
        midi_out: false,
        size: (1, 1),
        min_size: (1, 1),
    };

    fn spec() -> Spec {
        Spec {
            info: &INFO,
            params: &[],
            meters: &[],
            assets: &[],
            key_in: 0,
            category: kit::Category::Other,
        }
    }

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("kit-presets-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn a_name_becomes_a_file_a_file_system_will_take() {
        let cases = [
            ("Warm tape", Some("Warm tape.json")),
            ("Init (mine)", Some("Init (mine).json")),
            ("a/b\\c:d*e?", Some("abcde.json")),
            ("../..", None),
            ("   ", None),
        ];

        for (name, want) in cases {
            assert_eq!(file_name(name).as_deref(), want, "{name:?}");
        }
    }

    #[test]
    fn a_preset_saved_is_listed_and_one_deleted_is_gone() {
        let dir = scratch("round");
        let save =
            r#"{"type":"preset-save","name":"Mine","values":{"gain":0.25,"mode":7,"bad":"x"}}"#;

        let told: Value = serde_json::from_str(&handle(&spec(), &dir, save).unwrap()).unwrap();
        assert_eq!(told["type"], "presets");
        assert_eq!(told["list"][0]["name"], "Mine");
        // Kept from 0 to 1, and what is not a number is not kept.
        assert_eq!(
            told["list"][0]["values"],
            json!({ "gain": 0.25, "mode": 1.0 })
        );
        assert!(dir.join("Mine.json").is_file());

        let gone = handle(&spec(), &dir, r#"{"type":"preset-delete","name":"Mine"}"#).unwrap();
        assert_eq!(
            serde_json::from_str::<Value>(&gone).unwrap()["list"],
            json!([])
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn presets_come_back_by_name_and_other_files_are_skipped() {
        let dir = scratch("list");
        for name in ["beta", "Alpha"] {
            save(&spec(), &dir, name, &json!({ "gain": 0.5 })).unwrap();
        }
        fs::write(dir.join("notes.txt"), "not one").unwrap();
        fs::write(dir.join("broken.json"), "{").unwrap();

        let names: Vec<_> = list(&dir)
            .iter()
            .map(|p| p["name"].as_str().unwrap().to_owned())
            .collect();
        assert_eq!(names, ["Alpha", "beta"]);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn anything_but_a_preset_message_is_not_ours() {
        let dir = scratch("other");

        assert_eq!(
            handle(&spec(), &dir, r#"{"type":"set","id":1,"value":0.5}"#),
            None
        );
        assert_eq!(handle(&spec(), &dir, "nonsense"), None);
        assert_eq!(
            handle(&spec(), &dir, r#"{"type":"preset-save","name":"/"}"#),
            None
        );
        assert!(!dir.exists());
    }
}
