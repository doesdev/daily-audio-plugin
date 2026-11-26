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
    let ticks = Cell::new(0);
    Box::new(move || {
        let version = link.borrow().as_ref().map(|scope| scope.learn.version());
        let due = ticks.replace((ticks.get() + 1) % RETELL) == 0;
        if version.is_some() && (due || told.get() != version) {
            told.set(version);
            let link = link.borrow();
            return Some(bridge::learn(&spec, &link.as_ref()?.learn));
        }
        sound()
    })
}

/// Does what the page asks of MIDI learn: `learn` with a setting's id
/// has it wait for a controller, `learn` with none stops the waiting,
/// and `forget` frees a setting from its controller.
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
        let link = link.borrow();
        match (kind, link.as_ref(), index) {
            (Some("learn"), Some(scope), index) => scope.learn.listen(index),
            (Some("forget"), Some(scope), Some(index)) => scope.learn.bind(index, None),
            (Some("learn" | "forget"), _, _) => {}
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
        assert_eq!(feed(), None);

        scope.learn.listen(Some(1));
        assert_eq!(told(feed()).unwrap()["waiting"], 9);
        assert_eq!(scope.learn.heard(74), Some(1));
        let bound = told(feed()).unwrap();
        assert_eq!(bound["waiting"], Value::Null);
        assert_eq!(bound["map"], serde_json::json!({ "9": 74 }));

        // Nothing changes, yet within a second it is said again.
        let again = (0..RETELL).filter_map(|_| told(feed())).count();
        assert_eq!(again, 1);
    }
}
