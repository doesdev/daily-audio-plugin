// The two decays of a note, worked out from the settings by the law the
// strings follow, as `foreseen` in src/lib.rs has it: each string one
// phasor, a period at a time, at the reference pitch. ui/test.js holds
// the two to the same numbers.

export const REFERENCE_HZ = 220

// How far the bridge pulls on what the strings do together each pass,
// so that dies in `prompt` seconds while the rest dies in `after`.
export const coupling = (prompt, after, hz) => {
  const extra = Math.max(0, 1 / prompt - 1 / after)
  return Math.min(0.5, Math.max(0, 0.5 * (1 - 10 ** (-3 * extra / hz))))
}

// What a wave keeps each pass, to die by 60 dB in `seconds`.
export const keeps = (seconds, hz) =>
  10 ** (-3 / (Math.max(seconds, 1e-3) * hz))

// The middle and the side every `every` periods from the strike to
// `seconds` on: [{ t, mid, side }], the first at the strike.
export const decays = (prompt, after, mistune, strike, seconds, every = 1) => {
  const hz = REFERENCE_HZ
  const alpha = coupling(prompt, after, hz)
  const g = keeps(after, hz)
  const turn = Math.PI * mistune / hz
  const [c, s] = [Math.cos(turn), Math.sin(turn)]
  const periods = Math.round(seconds * hz)
  const half = (x, y) => 0.5 * Math.hypot(x, y)
  const step = ({ a, b }) => {
    const ra = [g * (a[0] * c + a[1] * s), g * (a[1] * c - a[0] * s)]
    const rb = [g * (b[0] * c - b[1] * s), g * (b[1] * c + b[0] * s)]
    const pull = [alpha * (ra[0] + rb[0]), alpha * (ra[1] + rb[1])]
    return {
      a: [ra[0] - pull[0], ra[1] - pull[1]],
      b: [rb[0] - pull[0], rb[1] - pull[1]]
    }
  }
  const seen = ({ a, b }, n) => ({
    t: n / hz,
    mid: half(a[0] + b[0], a[1] + b[1]),
    side: half(a[0] - b[0], a[1] - b[1])
  })
  const start = { a: [1 + strike, 0], b: [1 - strike, 0] }
  return Array.from({ length: periods }).reduce(({ now, out }, _, i) => {
    const next = step(now)
    const n = i + 1
    return {
      now: next,
      out: n % every === 0 || n === periods ? [...out, seen(next, n)] : out
    }
  }, { now: start, out: [seen(start, 0)] }).out
}

// The middle and the side `seconds` after a strike.
export const foreseen = (prompt, after, mistune, strike, seconds) => {
  const all = decays(prompt, after, mistune, strike, seconds, Infinity)
  const { mid, side } = all[all.length - 1]
  return [mid, side]
}

// The first time the side is louder than the middle, or null.
export const takesOver = (points) =>
  points.find(({ mid, side }) => side > mid)?.t ?? null
