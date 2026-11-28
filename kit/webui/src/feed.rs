//! The sound on its way to the page: what an editor's tick sends when
//! there is something new to draw.

use crate::bridge;
use kit::{Scope, Spec};
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::Arc;

/// The scope an editor draws from, once it has one: at once in CLAP,
/// and in VST3 when the host has connected the processor.
pub type ScopeLink = Rc<RefCell<Option<Arc<Scope>>>>;

/// Asked each tick: a message for the page, or `None` for nothing new.
pub type Feed = Box<dyn Fn() -> Option<String>>;

/// How many samples go to the page each way: a wave to draw, and a
/// spectrum of half as many bins.
pub const SHOWN: usize = 1024;

/// Asked with each message from the page: whether it was one of ours,
/// and so dealt with.
pub type Hear = Box<dyn Fn(&str) -> bool>;

// How many ticks pass between tellings of what has been learned when
// nothing has changed: a page that opened late still hears within a
// second.
const RETELL: u32 = 30;

/// The feed for everything the audio thread and the editor share:
/// what has been learned, whenever it changes and now and then besides,
/// and otherwise the sound itself while audio runs.
pub fn live(spec: Spec, link: ScopeLink) -> Feed {
    let sound = signal(link.clone());
    let told = Cell::new(None);
    let tied = Cell::new(None);
    let ticks = Cell::new(0);
    Box::new(move || {
        let versions = link
            .borrow()
            .as_ref()
            .map(|scope| (scope.learn.version(), scope.links.version()));
        let tick = ticks.replace((ticks.get() + 1) % RETELL);
        let (learnt, linked) = (versions.map(|v| v.0), versions.map(|v| v.1));
        if learnt.is_some() && (tick == 0 || told.get() != learnt) {
            told.set(learnt);
            let link = link.borrow();
            return Some(bridge::learn(&spec, &link.as_ref()?.learn));
        }
        // Who is linked, half a turn after: one message a tick.
        if linked.is_some() && (tick == RETELL / 2 || tied.get() != linked) {
            tied.set(linked);
            let link = link.borrow();
            return Some(bridge::links(&spec, &link.as_ref()?.links));
        }
        sound()
    })
}

/// Does what the page asks of MIDI learn and of linking. `learn`
/// with a setting's id has it wait for a controller, `learn` with
/// none stops the waiting, and `forget` frees a setting from its
/// controller. `tie` links a setting with the same one of the
/// instance `peer`; `untie` takes that peer's out of the group, or
/// with no peer this instance's own. `default` makes `plain` where a
/// setting starts from now on, for this plugin or, with `reach` as
/// `all`, for every DAP plugin.
pub fn learner(spec: Spec, link: ScopeLink) -> Hear {
    Box::new(move |text| {
        let Ok(msg) = serde_json::from_str::<serde_json::Value>(text) else {
            return false;
        };
        let kind = msg.get("type").and_then(|t| t.as_str());
        let index = msg
            .get("id")
            .and_then(|id| u32::try_from(id.as_u64()?).ok())
            .and_then(|id| spec.setting_of(id));
        let peer = msg
            .get("peer")
            .and_then(|p| p.as_str())
            .and_then(|p| p.parse::<u64>().ok());
        let key = index.map(|i| spec.params[i].key);
        let link = link.borrow();
        match (kind, link.as_ref(), index) {
            (Some("learn"), Some(scope), index) => scope.learn.listen(index),
            (Some("forget"), Some(scope), Some(index)) => scope.learn.bind(index, None),
            (Some("tie"), Some(scope), Some(index)) => {
                if let (Some(peer), Some(key)) = (peer, key) {
                    scope.links.tie(index, key, peer);
                }
            }
            (Some("untie"), Some(scope), Some(index)) => {
                let alone = msg.get("peer").is_none();
                if peer.is_some() || alone {
                    scope.links.untie(index, key.unwrap_or(""), peer);
                }
            }
            (Some("default"), Some(scope), Some(_)) => {
                let plain = msg.get("plain").and_then(|p| p.as_f64());
                let all = msg.get("reach").and_then(|r| r.as_str()) == Some("all");
                let plugin = (!all).then_some(spec.info.name);
                if let (Some(plain), Some(key)) = (plain, key) {
                    scope.links.make_default(plugin, key, plain);
                }
            }
            (Some("learn" | "forget" | "tie" | "untie" | "default"), _, _) => {}
            _ => return false,
        }
        true
    })
}

/// The feed that sends the latest of `link`'s scope while audio runs,
/// and nothing while it does not.
pub fn signal(link: ScopeLink) -> Feed {
    let last = Cell::new(0);
    let samples = RefCell::new((Vec::new(), Vec::new()));
    Box::new(move || {
        let link = link.borrow();
        let scope = link.as_ref()?;
        let now = scope.written();
        if now == last.replace(now) {
            return None;
        }
        let mut samples = samples.borrow_mut();
        let (input, output) = &mut *samples;
        scope.latest(SHOWN, input, output);
        Some(bridge::signal(scope.rate(), input, output))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    #[test]
    fn it_sends_the_sound_while_it_moves_and_nothing_when_it_stops() {
        let link: ScopeLink = Rc::default();
        let feed = signal(link.clone());
        // No scope yet: a VST3 editor opened before the host connects.
        assert_eq!(feed(), None);

        let scope = Arc::new(Scope::default());
        scope.set_rate(48_000.0);
        *link.borrow_mut() = Some(scope.clone());
        assert_eq!(feed(), None);

        scope.push(0.5, -0.25);
        scope.push(1.0, 3.0);
        let told: Value = serde_json::from_str(&feed().expect("a message")).unwrap();
        assert_eq!(told["type"], "signal");
        assert_eq!(told["rate"], 48_000.0);
        let scale = told["scale"].as_f64().unwrap();
        let heard = |way: &str| -> Vec<f64> {
            told[way]
                .as_array()
                .unwrap()
                .iter()
                .map(|v| v.as_f64().unwrap() / scale)
                .collect()
        };
        assert_eq!(heard("input"), [0.5, 1.0]);
        // Past twice full scale it is held there, not wrapped.
        assert_eq!(heard("output"), [-0.25, 2.0]);

        assert_eq!(feed(), None);
    }

    use kit::{Info, Role, list, range};

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
            range(4, "time", "Time", "", (0.0, 500.0, 120.0), "ms"),
            list(9, "mode", "Mode", "", &["Tape", "Clean"], 0),
        ],
        meters: &[],
        assets: &[],
        key_in: 0,
        category: kit::Category::Other,
    };

    fn linked() -> (ScopeLink, Arc<Scope>) {
        let scope = Arc::new(Scope::default());
        (Rc::new(RefCell::new(Some(scope.clone()))), scope)
    }

    #[test]
    fn the_page_asks_a_setting_to_learn_by_its_id_and_to_forget() {
        let (link, scope) = linked();
        let hear = learner(SPEC, link);

        assert!(hear(r#"{"type":"learn","id":9}"#));
        assert_eq!(scope.learn.waiting(), Some(1));
        assert!(hear(r#"{"type":"learn","id":null}"#));
        assert_eq!(scope.learn.waiting(), None);

        scope.learn.bind(0, Some(74));
        assert!(hear(r#"{"type":"forget","id":4}"#));
        assert_eq!(scope.learn.cc(0), None);

        // An id that is no setting's waits for nothing, and anything
        // else is not ours to answer.
        assert!(hear(r#"{"type":"learn","id":77}"#));
        assert_eq!(scope.learn.waiting(), None);
        assert!(!hear(r#"{"type":"set","id":4,"value":0.5}"#));
        assert!(!hear("nonsense"));
    }

    #[test]
    fn what_is_learned_is_told_when_it_changes_and_again_now_and_then() {
        let (link, scope) = linked();
        let feed = live(SPEC, link);
        let told = |msg: Option<String>| -> Option<Value> { serde_json::from_str(&msg?).ok() };

        let first = told(feed()).expect("told at once");
        assert_eq!(first["type"], "learn");
        assert_eq!(first["waiting"], Value::Null);
        // Then who is linked, and then nothing.
        assert_eq!(told(feed()).expect("and links")["type"], "links");
        assert_eq!(feed(), None);

        scope.learn.listen(Some(1));
        assert_eq!(told(feed()).unwrap()["waiting"], 9);
        assert_eq!(scope.learn.heard(74), Some(1));
        let bound = told(feed()).unwrap();
        assert_eq!(bound["waiting"], Value::Null);
        assert_eq!(bound["map"], serde_json::json!({ "9": 74 }));

        // Nothing changes, yet within a second it is said again.
        let again = (0..RETELL)
            .filter_map(|_| told(feed()))
            .filter(|msg| msg["type"] == "learn")
            .count();
        assert_eq!(again, 1);
    }

    #[test]
    fn the_page_is_told_who_is_linked_and_can_link_and_unlink() {
        let scope = Arc::new(Scope::default());
        let link: ScopeLink = Rc::new(RefCell::new(Some(scope.clone())));
        let hear = learner(SPEC, link.clone());
        let feed = live(SPEC, link);
        // Until something says `links`.
        let told = || {
            (0..80).find_map(|_| {
                let msg: Value = serde_json::from_str(&feed()?).ok()?;
                (msg["type"] == "links").then_some(msg)
            })
        };

        assert!(hear(r#"{"type":"tie","id":4,"peer":"42"}"#));
        let group = scope.links.group(0);
        assert_ne!(group, 0);
        let msg = told().expect("told");
        assert_eq!(msg["groups"]["4"], group.to_string());
        assert_eq!(msg["peers"], serde_json::json!([]));

        assert!(hear(r#"{"type":"untie","id":4}"#));
        assert_eq!(scope.links.group(0), 0);
        assert_eq!(told().expect("told")["groups"], serde_json::json!({}));
        // A default is taken as said; with nowhere to keep it, as here,
        // nothing is written.
        assert!(hear(r#"{"type":"default","id":4,"reach":"all","plain":1}"#));
        assert!(hear(r#"{"type":"default","id":4}"#));
        // A peer given as anything but its number in text links nothing.
        assert!(hear(r#"{"type":"tie","id":4,"peer":7}"#));
        assert!(hear(r#"{"type":"untie","id":4,"peer":7}"#));
        assert_eq!(scope.links.group(0), 0);
    }
}
