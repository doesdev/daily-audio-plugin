// What Hand-Me-Down's page works out for itself to draw the family
// line: how much of a note is its own hit, where notes started, and
// which earlier note each one's body came from.

// How much of the note's own sound is its hit, `i` samples in, as
// src/lib.rs has it in `hit_weight`: rising over the crossfade, held
// for `hit`, then fading over `fade`. tests/sound.rs holds the sound
// to the same numbers as ui/test.js holds this.
export const hitWeight = (i, cross, hit, fade) => {
  const rise = Math.min(1, i / Math.max(1, cross))
  const fall = i < hit ? 1 : Math.max(0, 1 - (i - hit) / Math.max(1, fade))
  return rise * fall
}

// The age meter climbs from each note's start and drops at the next,
// so a note started wherever a reading is lower than the one before.
// The indexes into the trail where notes started, oldest first.
export const startsIn = (ages, by = 0.002) => ages
  .map((age, i) => (i > 0 && age < ages[i - 1] - by ? i : -1))
  .filter((i) => i >= 0)

// For each note, the index of the note its body came from, `back`
// notes before it, or null when there was none that far back.
export const heirs = (count, back) => Array.from({ length: count },
  (_, k) => (k - back >= 0 ? k - back : null))

// The level of a plucked note `t` seconds in, for the picture drawn
// when no sound is running: a fall like a string's.
export const fall = (t) => Math.exp(-t / 0.35)

// The line drawn when no sound is running: when each note starts, in
// seconds, how hard it is struck, and how long the picture runs.
export const RESTING = {
  starts: [0.24, 0.54, 0.74, 1.1, 1.36, 1.78, 2, 2.32],
  levels: [0.8, 0.65, 0.9, 0.7, 0.85, 0.6, 0.95, 0.75],
  seconds: 2.72
}

// The body heard `t` seconds into note `k` of a line: the earlier
// note's, from the same point in its life, while that note still
// rang; then the note's own. Its level, and whether it was handed
// down.
export const bodyAt = (line, k, back, t) => {
  const { starts, levels, seconds } = line
  const from = k - back
  const length = (j) => (starts[j + 1] ?? seconds) - starts[j]
  if (from >= 0 && t < length(from)) {
    return { level: levels[from] * fall(t), handed: true }
  }
  return { level: levels[k] * fall(t), handed: false }
}

// How high a level stands on the glass, as a fraction of its half: on
// a scale of decibels from -40 to +12, so a body turned down by 24 dB
// is still seen and one turned up by 12 just fills it.
export const heightOf = (level) => level <= 0
  ? 0
  : Math.min(1, Math.max(0, (20 * Math.log10(level) + 40) / 52))
