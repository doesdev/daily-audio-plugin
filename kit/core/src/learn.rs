//! MIDI learn: which controller moves which setting. A person asks a
//! setting to learn, moves a knob on their hardware, and from then on
//! that knob is that setting. The editor asks, the audio thread hears
//! the controller and binds it, and neither waits for the other.

use std::sync::atomic::{AtomicI16, AtomicI32, AtomicU64, Ordering};

/// How many settings can each have a controller.
pub const LEARNABLE: usize = 256;

const NONE: i16 = -1;

pub struct Learn {
    // The controller of each setting, by the setting's index.
    ccs: Box<[AtomicI16]>,
    // The setting waiting for a controller to be moved.
    waiting: AtomicI32,
    // Moves whenever anything above does, for an editor to notice.
    version: AtomicU64,
}

impl Default for Learn {
    fn default() -> Self {
        Self {
            ccs: (0..LEARNABLE).map(|_| AtomicI16::new(NONE)).collect(),
            waiting: AtomicI32::new(-1),
            version: AtomicU64::new(0),
        }
    }
}

impl Learn {
    fn changed(&self) {
        self.version.fetch_add(1, Ordering::AcqRel);
    }

    /// From the editor: setting `index` takes the next controller that
    /// moves, or with `None` nothing is waiting any more.
    pub fn listen(&self, index: Option<usize>) {
        let waiting = index.filter(|&i| i < LEARNABLE).map_or(-1, |i| i as i32);
        self.waiting.store(waiting, Ordering::Release);
        self.changed();
    }

    /// The setting waiting for a controller.
    pub fn waiting(&self) -> Option<usize> {
        usize::try_from(self.waiting.load(Ordering::Acquire)).ok()
    }

    /// The controller setting `index` answers to.
    pub fn cc(&self, index: usize) -> Option<u8> {
        let cc = self.ccs.get(index)?.load(Ordering::Relaxed);
        u8::try_from(cc).ok()
    }

    /// Binds setting `index` to `cc`, or frees it with `None`. A
    /// controller moves one setting: binding it takes it from any other.
    pub fn bind(&self, index: usize, cc: Option<u8>) {
        if index >= LEARNABLE {
            return;
        }
        let cc = cc.filter(|&cc| cc < 128).map_or(NONE, i16::from);
        for (at, slot) in self.ccs.iter().enumerate() {
            if at == index {
                slot.store(cc, Ordering::Relaxed);
            } else if cc != NONE && slot.load(Ordering::Relaxed) == cc {
                slot.store(NONE, Ordering::Relaxed);
            }
        }
        self.changed();
    }

    /// From the audio thread: controller `cc` moved. If a setting was
    /// waiting, it is bound. The setting the controller now moves, if
    /// any. Never allocates, locks or waits.
    pub fn heard(&self, cc: u8) -> Option<usize> {
        if cc >= 128 {
            return None;
        }
        let waiting = self.waiting.swap(-1, Ordering::AcqRel);
        if let Ok(index) = usize::try_from(waiting) {
            self.bind(index, Some(cc));
        }
        self.ccs
            .iter()
            .position(|slot| slot.load(Ordering::Relaxed) == i16::from(cc))
    }

    /// Moves whenever a binding or what is waiting changes.
    pub fn version(&self) -> u64 {
        self.version.load(Ordering::Acquire)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_waiting_setting_takes_the_next_controller_and_keeps_it() {
        let learn = Learn::default();
        assert_eq!(learn.heard(74), None);

        learn.listen(Some(3));
        assert_eq!(learn.waiting(), Some(3));
        assert_eq!(learn.heard(74), Some(3));

        // Bound, no longer waiting, and it answers from now on.
        assert_eq!(learn.waiting(), None);
        assert_eq!(learn.cc(3), Some(74));
        assert_eq!(learn.heard(74), Some(3));
        assert_eq!(learn.heard(75), None);
    }

    #[test]
    fn a_controller_moves_one_setting_and_a_setting_can_be_freed() {
        let learn = Learn::default();
        learn.bind(1, Some(20));
        learn.bind(2, Some(20));

        assert_eq!((learn.cc(1), learn.cc(2)), (None, Some(20)));
        learn.bind(2, None);
        assert_eq!(learn.heard(20), None);
    }

    #[test]
    fn asking_can_be_taken_back_and_every_change_is_noticed() {
        let learn = Learn::default();
        let before = learn.version();
        learn.listen(Some(0));
        learn.listen(None);

        assert_eq!(learn.heard(1), None);
        assert_eq!(learn.cc(0), None);
        assert!(learn.version() > before);
    }

    #[test]
    fn what_cannot_be_a_setting_or_a_controller_is_left_alone() {
        let learn = Learn::default();
        learn.listen(Some(LEARNABLE));
        learn.bind(LEARNABLE, Some(1));
        learn.bind(0, Some(200));

        assert_eq!(learn.waiting(), None);
        assert_eq!(learn.cc(0), None);
        assert_eq!(learn.heard(200), None);
    }
}
