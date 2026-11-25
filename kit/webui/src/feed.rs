//! The sound on its way to the page: what an editor's tick sends when
//! there is something new to draw.

use crate::bridge;
use kit::Scope;
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
}
