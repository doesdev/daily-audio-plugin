// Aftersound's page works out the two decays itself, to draw them.
// These hold it to the same numbers as the sound's own `foreseen`
// (tests/sound.rs): change one and change the other.

import test from 'mvt'
import { decays, foreseen, takesOver } from './foreseen.js'

const close = (got, want) => Math.abs(got / want - 1) < 0.01

test('foreseen gives the numbers the sound is held to', (assert) => {
  const cases = [
    // prompt, after, mistune, strike, seconds, mid, side
    [0.6, 7, 0.5, 0, 0.25, 0.0435682, 0.108273],
    [0.6, 7, 0.5, 0, 1, 0.00698757, 0.0469545],
    [0.6, 7, 0.5, 0, 3, 0.000602003, 0.00403794],
    [0.3, 20, 1.5, 0.4, 1, 0.0253562, 0.122723],
    [1.2, 4, 2.5, -0.2, 0.8, 0.0518341, 0.0143078]
  ]

  cases.forEach(([prompt, after, mistune, strike, t, mid, side]) => {
    const [m, s] = foreseen(prompt, after, mistune, strike, t)
    assert.true(close(m, mid) && close(s, side), `${t} s: ${m} ${s}`)
  })
})

test('a note starts whole in the middle and the side takes over', (assert) => {
  const points = decays(0.6, 7, 0.5, 0, 3, 11)
  const at = takesOver(points)

  assert.deepEqual([points[0].mid, points[0].side], [1, 0])
  assert.true(at > 0 && at < 0.5, `takes over at ${at}`)
  assert.is(takesOver(decays(0.6, 7, 0, 0, 3, 11)), null)
})

test('decays are drawn every so many periods, and to the end', (assert) => {
  const points = decays(0.6, 7, 0.5, 0, 1, 100)

  assert.deepEqual(points.map(({ t }) => Math.round(t * 220)),
    [0, 100, 200, 220])
})
