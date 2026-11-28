// What Ironside's page works out for itself, to draw: where the sides'
// corner stands, how open the core is at a given flux, and what the
// sides' path does to each frequency. Each is held to the sound by a
// test in tests/sound.rs with the same numbers as ui/test.js.

// As src/lib.rs has it: Fill 100% pushes the sides' winding this hard.
export const MOST_FILL = 16

// The sides' corner in hertz for a core `open` (one empty, toward
// nought full), at `rest` when empty and never above `ceiling`.
export const cornerOf = (rest, ceiling, open) =>
  Math.min(rest / Math.max(open, 1e-6), Math.max(ceiling, rest))

// The slope of the core's curve at flux `x`: how much a second winding
// gets through there. As kit_dsp::iron_slope.
export const ironSlope = (x, push, iron = 0) => {
  if (push < 1e-4) return 1
  const u = x * push
  const u4 = u ** 4
  const nickel = 1 / (Math.pow(1 + u4, 0.25) * (1 + u4))
  const t = Math.tanh(u)
  return nickel + (1 - t * t - nickel) * iron
}

// The flux at which the core is `open`: the slope falls as the flux
// rises, so halving the span finds it.
export const fluxAt = (open, push, iron = 0) => {
  if (push < 1e-4 || open >= 1) return 0
  const most = 8 / push
  const halve = (lo, hi, n) => {
    if (n === 0) return (lo + hi) / 2
    const mid = (lo + hi) / 2
    return ironSlope(mid, push, iron) > open
      ? halve(mid, hi, n - 1)
      : halve(lo, mid, n - 1)
  }
  return halve(0, most, 40)
}

// What the sides' path does to `hz`, in dB, with its corner at
// `corner` and the effect mixed in by `mix` (0 to 1): a choke across
// the path is a first-order high-pass.
export const keptDb = (hz, corner, mix = 1) => {
  const dry = corner * (1 - mix)
  const kept = Math.sqrt(dry * dry + hz * hz) /
    Math.sqrt(corner * corner + hz * hz)
  return 20 * Math.log10(Math.max(kept, 1e-9))
}
