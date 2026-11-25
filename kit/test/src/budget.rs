//! The realtime budget every plugin is held to: a counting allocator,
//! and the cost of processing a signal as a fraction of real time.

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
use std::sync::RwLock;
use std::time::{Duration, Instant};

/// Mean share of real time one instance may spend, at any rate and
/// block: a tenth of a core.
pub const MEAN_CEILING: f64 = 0.10;
/// Share of its own length the slowest block may take.
pub const WORST_CEILING: f64 = 0.50;

pub const RATES: [f32; 3] = [44_100.0, 48_000.0, 96_000.0];
pub const BLOCKS: [usize; 2] = [64, 4096];

// Timed passes after one warm-up; each block's time is the median.
const RUNS: usize = 5;

// ---- Allocations

/// Counts allocations made on a thread while it is counting.
pub struct Counting;

thread_local! {
    static COUNT: Cell<Option<usize>> = const { Cell::new(None) };
}

fn note() {
    let _ = COUNT.try_with(|c| c.set(c.get().map(|n| n + 1)));
}

unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        note();
        unsafe { System.alloc(layout) }
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        note();
        unsafe { System.alloc_zeroed(layout) }
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        note();
        unsafe { System.realloc(ptr, layout, size) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        note();
        unsafe { System.dealloc(ptr, layout) }
    }
}

/// Allocations and frees `f` made on this thread.
pub fn allocations(f: impl FnOnce()) -> usize {
    // Never beside a timing pass, which it would slow.
    let _shared = ALONE.read().unwrap_or_else(|e| e.into_inner());
    COUNT.set(Some(0));
    f();
    COUNT.replace(None).unwrap_or(0)
}

// ---- Cost

/// Processing cost as a fraction of real time.
#[derive(Debug, Clone, Copy)]
pub struct Cost {
    /// Over the whole signal: the share of a core one instance takes.
    pub mean: f64,
    /// The slowest block against its own length.
    pub worst: f64,
}

impl Cost {
    pub fn within(&self) -> bool {
        self.mean < MEAN_CEILING && self.worst < WORST_CEILING
    }
}

// Timing passes share the machine with nothing else from their binary:
// a timing pass holds this alone, and every allocation count holds it
// shared, so no test's processing runs beside a pass.
static ALONE: RwLock<()> = RwLock::new(());

/// Runs fresh instances from `make` over `audio` in `block`-sized
/// blocks, `process` doing one block.
pub fn cost<S>(
    rate: f32,
    block: usize,
    audio: &[f32],
    mut make: impl FnMut() -> S,
    mut process: impl FnMut(&mut S, &[f32]),
) -> Cost {
    let _alone = ALONE.write().unwrap_or_else(|e| e.into_inner());
    let chunks: Vec<&[f32]> = audio.chunks(block).collect();
    let mut times = vec![vec![Duration::ZERO; RUNS]; chunks.len()];

    for run in 0..=RUNS {
        let mut instance = make();
        for (i, chunk) in chunks.iter().enumerate() {
            let at = Instant::now();
            process(&mut instance, chunk);
            let took = at.elapsed();
            if run > 0 {
                times[i][run - 1] = took;
            }
        }
    }

    let medians: Vec<f64> = times
        .iter_mut()
        .map(|t| {
            t.sort();
            t[RUNS / 2].as_secs_f64()
        })
        .collect();
    let mean = medians.iter().sum::<f64>() / (audio.len() as f64 / rate as f64);
    let worst = medians
        .iter()
        .zip(&chunks)
        .map(|(t, c)| t / (c.len() as f64 / rate as f64))
        .fold(0.0, f64::max);
    Cost { mean, worst }
}

/// The cost at every rate and block, with any over the ceilings, as
/// lines a failing test prints.
pub fn over(mut at: impl FnMut(f32, usize) -> Cost) -> Vec<String> {
    let mut over = Vec::new();
    for rate in RATES {
        for block in BLOCKS {
            let cost = at(rate, block);
            let line = format!(
                "{rate} Hz, {block} block: mean {:.2}%, worst block {:.1}%",
                cost.mean * 100.0,
                cost.worst * 100.0
            );
            eprintln!("{line}");
            if !cost.within() {
                over.push(line);
            }
        }
    }
    over
}
