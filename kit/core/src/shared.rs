//! Settings and meters as every thread may read them: the host's
//! automation, the editor and the audio thread meet here, with no lock.

use crate::{Scope, Spec};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU8, AtomicU64, Ordering};

// What the editor has done to a setting, for the host to hear about.
pub const BEGIN: u8 = 1;
pub const SET: u8 = 2;
pub const END: u8 = 4;

pub struct Shared {
    values: Box<[AtomicU64]>,
    meters: Box<[AtomicU64]>,
    edits: Box<[AtomicU8]>,
    dirty: AtomicBool,
    scope: Arc<Scope>,
}

impl Shared {
    pub fn new(spec: &Spec) -> Self {
        Self {
            values: spec
                .defaults()
                .into_iter()
                .map(|v| AtomicU64::new(v.to_bits()))
                .collect(),
            meters: spec.meters.iter().map(|_| AtomicU64::new(0)).collect(),
            edits: spec.params.iter().map(|_| AtomicU8::new(0)).collect(),
            dirty: AtomicBool::new(true),
            scope: Arc::default(),
        }
    }

    /// The sound itself, for the runner to write and an editor to draw.
    pub fn scope(&self) -> Arc<Scope> {
        self.scope.clone()
    }

    /// Setting `index`, normalized.
    pub fn value(&self, index: usize) -> Option<f64> {
        let slot = self.values.get(index)?;
        Some(f64::from_bits(slot.load(Ordering::Relaxed)))
    }

    pub fn set(&self, index: usize, norm: f64) {
        if let Some(slot) = self.values.get(index) {
            slot.store(norm.clamp(0.0, 1.0).to_bits(), Ordering::Relaxed);
            self.dirty.store(true, Ordering::Release);
        }
    }

    /// Every setting. Allocates: not for the audio thread.
    pub fn values(&self) -> Vec<f64> {
        (0..self.values.len())
            .filter_map(|i| self.value(i))
            .collect()
    }

    /// Every setting into `out`, with no allocation.
    pub fn snapshot(&self, out: &mut [f64]) {
        for (slot, o) in self.values.iter().zip(out) {
            *o = f64::from_bits(slot.load(Ordering::Relaxed));
        }
    }

    pub fn set_all(&self, values: &[f64]) {
        for (i, &v) in values.iter().enumerate() {
            self.set(i, v);
        }
    }

    /// From the audio thread: MIDI controller `cc` moved to `value`, 0
    /// to 127. The setting that has learned it, if any, follows, and
    /// the host hears of it as it would of a move in the editor.
    pub fn controller(&self, cc: u8, value: u8) {
        if let Some(index) = self
            .scope
            .learn
            .heard(cc)
            .filter(|&i| i < self.values.len())
        {
            self.set(index, f64::from(value.min(127)) / 127.0);
            self.edit(index, BEGIN | SET | END);
        }
    }

    /// The controller each setting has learned, for saving.
    pub fn learnt(&self) -> Vec<Option<u8>> {
        (0..self.values.len())
            .map(|i| self.scope.learn.cc(i))
            .collect()
    }

    /// Puts back what was saved.
    pub fn set_learnt(&self, ccs: &[Option<u8>]) {
        for (index, &cc) in ccs.iter().enumerate() {
            self.scope.learn.bind(index, cc);
        }
    }

    /// The group each setting is linked in, for saving.
    pub fn links(&self) -> Vec<u64> {
        (0..self.values.len())
            .map(|i| self.scope.links.group(i))
            .collect()
    }

    /// Puts back what was saved.
    pub fn set_links(&self, groups: &[u64]) {
        for (index, &group) in groups.iter().enumerate() {
            self.scope.links.set_group(index, group);
        }
    }

    /// Whether a setting moved since this was last asked.
    pub fn take_dirty(&self) -> bool {
        self.dirty.swap(false, Ordering::Acquire)
    }

    /// From the editor: part of a gesture on setting `index`.
    pub fn edit(&self, index: usize, what: u8) {
        if let Some(slot) = self.edits.get(index) {
            slot.fetch_or(what, Ordering::AcqRel);
        }
    }

    /// The gestures on setting `index` since last asked.
    pub fn take_edit(&self, index: usize) -> u8 {
        self.edits
            .get(index)
            .map_or(0, |slot| slot.swap(0, Ordering::AcqRel))
    }

    pub fn meter(&self, index: usize) -> f64 {
        self.meters
            .get(index)
            .map_or(0.0, |m| f64::from_bits(m.load(Ordering::Relaxed)))
    }

    pub fn set_meter(&self, index: usize, value: f64) {
        if let Some(slot) = self.meters.get(index) {
            slot.store(value.to_bits(), Ordering::Relaxed);
        }
    }

    pub fn meters(&self) -> usize {
        self.meters.len()
    }
}
