//! What the docs tools need from a plugin, as the code makes it: the
//! hello its page starts from, and one per scene, so a screenshot shows
//! the same text a host would. Each plugin has an example that calls
//! `main`; `scripts/shots.js` runs it.

use crate::bridge;
use kit::{Plugin, Spec};
use serde_json::{Map, Value, json};
use std::io::Read;

fn pick(spec: &Spec, key: &str, value: &Value) -> Result<(usize, f64), String> {
    let index = spec
        .rows()
        .position(|p| p.key == key)
        .ok_or_else(|| format!("no setting or meter has the key {key:?}"))?;
    let p = spec.row(index).ok_or("no such row")?;
    let norm = match value {
        Value::String(text) => p
            .parse(text)
            .ok_or_else(|| format!("{key} cannot be {text:?}"))?,
        Value::Bool(on) => f64::from(u8::from(*on)),
        other => {
            let plain = other
                .as_f64()
                .ok_or_else(|| format!("{key}: not a value"))?;
            p.normalized(plain)
        }
    };
    Ok((index, norm))
}

/// The plugin's hello at its defaults, and for each scene in `scenes`
/// (`[{ "name": "...", "values": { key: value } }]`, a value being a
/// number in the setting's own terms, a choice's name, or a meter's
/// fraction) the hello with those values.
pub fn dump(spec: &Spec, scenes: &Value) -> Result<Value, String> {
    let mut defaults = spec.defaults();
    defaults.resize(spec.total(), 0.0);
    let mut out = Map::new();
    for scene in scenes.as_array().map(Vec::as_slice).unwrap_or_default() {
        let name = scene["name"].as_str().ok_or("a scene with no name")?;
        let mut values = defaults.clone();
        for (key, value) in scene["values"].as_object().into_iter().flatten() {
            let (index, norm) = pick(spec, key, value).map_err(|e| format!("{name}: {e}"))?;
            values[index] = norm;
        }
        let hello: Value =
            serde_json::from_str(&bridge::hello(spec, &values)).map_err(|e| e.to_string())?;
        out.insert(name.to_string(), hello);
    }
    let hello: Value =
        serde_json::from_str(&bridge::hello(spec, &defaults)).map_err(|e| e.to_string())?;
    Ok(json!({
        "hello": hello,
        "scenes": out,
        "paths": bridge::paths(spec),
        "size": [spec.info.size.0, spec.info.size.1],
    }))
}

/// For a plugin's `examples/spec.rs`: prints `dump` as JSON, reading
/// scenes from standard input when run with `--scenes`. With `--asset
/// PATH` it prints that file of the page instead.
pub fn main<P: Plugin>() {
    let spec = Spec::of::<P>();
    let args: Vec<String> = std::env::args().skip(1).collect();
    if let Some(path) = args
        .iter()
        .position(|a| a == "--asset")
        .and_then(|i| args.get(i + 1))
    {
        match bridge::asset(&spec, path) {
            Some((_, body)) => print!("{body}"),
            None => {
                eprintln!("no asset {path}");
                std::process::exit(1);
            }
        }
        return;
    }

    let mut text = String::from("[]");
    if args.iter().any(|a| a == "--scenes") {
        text.clear();
        if let Err(e) = std::io::stdin().read_to_string(&mut text) {
            eprintln!("could not read the scenes: {e}");
            std::process::exit(1);
        }
    }
    let scenes: Value = serde_json::from_str(&text).unwrap_or_else(|e| {
        eprintln!("the scenes are not JSON: {e}");
        std::process::exit(1);
    });
    match dump(&spec, &scenes) {
        Ok(out) => println!("{out}"),
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kit::{Info, Role, list, meter, range};

    const INFO: Info = Info {
        name: "Thing",
        vendor: "kit",
        url: "",
        description: "",
        version: "0.1.0",
        clap_id: "dev.kit.thing",
        vst3_processor: [0; 4],
        vst3_controller: [1; 4],
        role: Role::Effect,
        audio_in: 2,
        audio_out: 2,
        midi_in: false,
        midi_out: false,
        size: (640, 360),
        min_size: (1, 1),
    };
    const SPEC: Spec = Spec {
        info: &INFO,
        params: &[
            range(0, "time", "Time", "", (0.0, 500.0, 120.0), "ms").decimals(0),
            list(1, "mode", "Mode", "", &["Tape", "Clean"], 0),
        ],
        meters: &[meter(2, "out", "Output")],
        assets: &[],
        key_in: 0,
    };

    #[test]
    fn a_scene_sets_values_by_key_in_their_own_terms() {
        let scenes = json!([
            { "name": "long", "values": { "time": 400, "mode": "Clean", "out": 0.5 } },
            { "name": "plain" },
        ]);
        let out = dump(&SPEC, &scenes).expect("dump");

        assert_eq!(out["hello"]["texts"], json!(["120 ms", "Tape"]));
        assert_eq!(out["scenes"]["long"]["texts"], json!(["400 ms", "Clean"]));
        assert_eq!(out["scenes"]["long"]["levelTexts"], json!(["-30 dB"]));
        assert_eq!(out["scenes"]["plain"]["texts"], json!(["120 ms", "Tape"]));
        assert_eq!(out["size"], json!([640, 360]));
    }

    #[test]
    fn says_what_is_wrong_with_a_scene() {
        let cases = [
            (
                json!([{ "name": "a", "values": { "tim": 1 } }]),
                "a: no setting",
            ),
            (
                json!([{ "name": "b", "values": { "mode": "Vinyl" } }]),
                "b: mode cannot be",
            ),
            (json!([{ "values": {} }]), "a scene with no name"),
        ];

        for (scenes, want) in cases {
            let err = dump(&SPEC, &scenes).expect_err("an error");
            assert!(err.starts_with(want), "{err}");
        }
    }
}
