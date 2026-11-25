//! Settings and meters as every thread may read them: the host's
//! automation, the editor and the audio thread meet here, with no lock.

use crate::Spec;
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
        }
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
