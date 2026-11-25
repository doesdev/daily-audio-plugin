//! What the page and the plugin say to each other, as JSON. The page
//! names a setting by the id hosts know it by; everything in here that
//! reaches Rust is an index into the spec.
//!
//! To the page: `hello` (the spec and every value, once it is ready),
//! `state` (settings moved outside the page) and `meters`. From the
//! page: `ready`, `blur`, and a gesture on a setting as `begin`, `set`
//! and `end`.

use kit::{Kind, Param, Spec};
use serde_json::{Value, json};
use std::borrow::Cow;

/// The protocol's version, for a page to check against its own.
pub const VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Action {
    /// The page has loaded and wants its hello.
    Ready,
    /// The page is done with the keyboard: give focus back to the host,
    /// so its shortcuts work.
    Blur,
    /// A gesture on a setting starts or ends, for host automation.
    Begin(usize),
    End(usize),
    /// A setting's new normalized value.
    Set(usize, f64),
}

/// An action from the page, or `None` for anything malformed or aimed
/// at something the page may not change.
pub fn parse(spec: &Spec, text: &str) -> Option<Action> {
    let msg: Value = serde_json::from_str(text).ok()?;
    let settable = |v: &Value| spec.setting_of(u32::try_from(v.as_u64()?).ok()?);

    match msg.get("type")?.as_str()? {
        "ready" => Some(Action::Ready),
        "blur" => Some(Action::Blur),
        "begin" => settable(msg.get("id")?).map(Action::Begin),
        "end" => settable(msg.get("id")?).map(Action::End),
        "set" => {
            let index = settable(msg.get("id")?)?;
            let value = msg.get("value")?.as_f64().filter(|v| v.is_finite())?;
            Some(Action::Set(index, value.clamp(0.0, 1.0)))
        }
        _ => None,
    }
}

fn describe(p: &Param) -> Value {
    let mut row = json!({
        "id": p.id,
        "key": p.key,
        "title": p.title,
        "short": p.short,
        "group": p.group,
        "about": p.about,
        "default": p.default_norm(),
        "steps": p.steps(),
    });
    let more = match p.kind {
        Kind::List(choices) => json!({ "kind": "list", "choices": choices }),
        Kind::Range {
            lo,
            hi,
            step,
            decimals,
            unit,
            skew,
        } => json!({
            "kind": "range",
            "lo": lo,
            "hi": hi,
            "step": step,
            "decimals": decimals,
            "unit": unit,
            "skew": skew,
        }),
        Kind::Level => json!({ "kind": "meter", "meter": "level" }),
        Kind::Fraction => json!({ "kind": "meter", "meter": "fraction" }),
    };
    if let (Some(row), Some(more)) = (row.as_object_mut(), more.as_object()) {
        row.extend(more.clone());
    }
    row
}

fn texts(rows: &[Param], values: &[f64]) -> Vec<String> {
    rows.iter()
        .zip(values)
        .map(|(p, &v)| p.display(v))
        .collect()
}

/// Everything the page needs to draw itself. `values` holds every
/// setting, then every meter.
pub fn hello(spec: &Spec, values: &[f64]) -> String {
    let (settings, meters) = values.split_at(spec.params.len().min(values.len()));
    json!({
        "type": "hello",
        "version": VERSION,
        "name": spec.info.name,
        "vendor": spec.info.vendor,
        "build": spec.info.version,
        "size": [spec.info.size.0, spec.info.size.1],
        "params": spec.params.iter().map(describe).collect::<Vec<_>>(),
        "meters": spec.meters.iter().map(describe).collect::<Vec<_>>(),
        "values": settings,
        "texts": texts(spec.params, settings),
        "levels": meters,
        "levelTexts": texts(spec.meters, meters),
    })
    .to_string()
}

/// Settings changed outside the page, by automation or the host.
pub fn state(spec: &Spec, values: &[f64]) -> String {
    json!({
        "type": "state",
        "values": values,
        "texts": texts(spec.params, values),
    })
    .to_string()
}

/// The meters as values and as the text the host shows for them.
pub fn meters(spec: &Spec, values: &[f64]) -> String {
    json!({
        "type": "meters",
        "levels": values,
        "levelTexts": texts(spec.meters, values),
    })
    .to_string()
}

/// The kit's own page, the same for every plugin.
const KIT: [kit::Asset; 4] = [
    (
        "/index.html",
        "text/html",
        include_str!("../../ui/index.html"),
    ),
    (
        "/kit/fonts.css",
        "text/css",
        include_str!("../../ui/fonts.css"),
    ),
    ("/kit/kit.css", "text/css", include_str!("../../ui/kit.css")),
    (
        "/kit/kit.js",
        "text/javascript",
        include_str!("../../ui/kit.js"),
    ),
];

/// The file of the page at `path` as (MIME type, text): the plugin's
/// own first, so it may replace any of the kit's.
pub fn asset(spec: &Spec, path: &str) -> Option<(&'static str, Cow<'static, str>)> {
    let path = if path == "/" { "/index.html" } else { path };
    spec.assets
        .iter()
        .chain(&KIT)
        .find(|(p, _, _)| *p == path)
        .map(|&(_, kind, body)| (kind, Cow::Borrowed(body)))
}

/// Every path the page can be asked for.
pub fn paths(spec: &Spec) -> Vec<&'static str> {
    let mut paths: Vec<_> = spec.assets.iter().chain(&KIT).map(|a| a.0).collect();
    paths.sort_unstable();
    paths.dedup();
    paths
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
    const SPEC: Spec = Spec {
        info: &INFO,
        params: &[
            range(4, "time", "Time", "Delay", (0.0, 500.0, 120.0), "ms")
                .step(5.0)
                .decimals(0)
                .about("How far behind the echo runs"),
            list(9, "mode", "Mode", "Delay", &["Tape", "Clean"], 0),
        ],
        meters: &[meter(2, "out", "Output")],
        assets: &[("/main.js", "text/javascript", "export default 1")],
    };

    #[test]
    fn parses_actions_by_host_id() {
        let cases = [
            (r#"{"type":"ready"}"#, Some(Action::Ready)),
            (r#"{"type":"blur"}"#, Some(Action::Blur)),
            (r#"{"type":"begin","id":9}"#, Some(Action::Begin(1))),
            (r#"{"type":"end","id":4}"#, Some(Action::End(0))),
            (
                r#"{"type":"set","id":9,"value":0.25}"#,
                Some(Action::Set(1, 0.25)),
            ),
            (
                r#"{"type":"set","id":4,"value":7}"#,
                Some(Action::Set(0, 1.0)),
            ),
            // A meter, an id no setting has, and junk.
            (r#"{"type":"set","id":2,"value":1}"#, None),
            (r#"{"type":"set","id":0,"value":1}"#, None),
            (r#"{"type":"set","id":4}"#, None),
            (r#"{"type":"set","id":4,"value":"loud"}"#, None),
            (r#"{"type":"launch"}"#, None),
            ("junk", None),
        ];

        for (text, want) in cases {
            assert_eq!(parse(&SPEC, text), want, "{text}");
        }
    }

    #[test]
    fn hello_carries_the_spec_and_every_value() {
        let values = [0.24, 1.0, 0.5];
        let msg: Value = serde_json::from_str(&hello(&SPEC, &values)).expect("json");

        assert_eq!(
            (&msg["type"], &msg["version"]),
            (&json!("hello"), &json!(VERSION))
        );
        assert_eq!(
            (&msg["name"], &msg["build"]),
            (&json!("Thing"), &json!("1.2.3"))
        );
        let time = &msg["params"][0];
        assert_eq!(
            (
                &time["id"],
                &time["key"],
                &time["kind"],
                &time["unit"],
                &time["steps"]
            ),
            (
                &json!(4),
                &json!("time"),
                &json!("range"),
                &json!("ms"),
                &json!(100)
            )
        );
        assert_eq!(time["about"], "How far behind the echo runs");
        assert_eq!(msg["params"][1]["choices"], json!(["Tape", "Clean"]));
        assert_eq!(msg["meters"][0]["meter"], "level");
        assert_eq!(msg["values"], json!([0.24, 1.0]));
        assert_eq!(msg["texts"], json!(["120 ms", "Clean"]));
        assert_eq!(msg["levels"], json!([0.5]));
        assert_eq!(msg["levelTexts"], json!(["-30 dB"]));
    }

    #[test]
    fn state_and_meters_carry_the_hosts_text() {
        let state: Value = serde_json::from_str(&state(&SPEC, &[0.0, 0.0])).expect("json");
        let meters: Value = serde_json::from_str(&meters(&SPEC, &[0.0])).expect("json");

        assert_eq!(state["texts"], json!(["0 ms", "Tape"]));
        assert_eq!(meters["levelTexts"], json!(["silent"]));
    }

    #[test]
    fn serves_the_kits_page_and_the_plugins_files() {
        let cases = [
            ("/", Some("text/html")),
            ("/index.html", Some("text/html")),
            ("/kit/kit.js", Some("text/javascript")),
            ("/kit/kit.css", Some("text/css")),
            ("/kit/fonts.css", Some("text/css")),
            ("/main.js", Some("text/javascript")),
            ("/test.js", None),
            ("/../Cargo.toml", None),
        ];

        for (path, want) in cases {
            assert_eq!(asset(&SPEC, path).map(|a| a.0), want, "{path}");
        }
        assert_eq!(asset(&SPEC, "/main.js").unwrap().1, "export default 1");
    }
}
