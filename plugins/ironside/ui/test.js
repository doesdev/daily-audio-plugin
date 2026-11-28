// Ironside's page works out the sides' corner, the core's slope and
// the sides' response itself, to draw them. Each is held to the same
// numbers as the sound (tests/sound.rs): change one and change the
// other.

import test from 'mvt'
import { cornerOf, fluxAt, ironSlope, keptDb } from './core.js'

test('the corner climbs as the core closes and stops at the ceiling',
  (assert) => {
    const cases = [
      [30, 1200, 1, 30],
      [30, 1200, 0.5, 60],
      [30, 1200, 0.1, 300],
      [30, 1200, 0.01, 1200],
      [30, 1200, 0, 1200],
      [100, 5000, 0.25, 400],
      [300, 100, 0.5, 300]
    ]
    cases.forEach(([rest, ceiling, open, want]) => {
      const got = cornerOf(rest, ceiling, open)
      assert.true(Math.abs(got - want) < 1e-3, `${rest} ${open}: ${got}`)
    })
  })

test('the core is as open at each flux as the sound has it', (assert) => {
  const cases = [
    [0, 4, 0, 1],
    [0.1, 8, 0, 0.651_074],
    [0.25, 8, 0, 0.028_969],
    [0.25, 8, 1, 0.070_651],
    [0.1, 4, 1, 0.855_639],
    [0.5, 0, 1, 1]
  ]
  cases.forEach(([x, push, iron, want]) => {
    const got = ironSlope(x, push, iron)
    assert.true(Math.abs(got - want) < 1e-5, `${x} ${push} ${iron}: ${got}`)
  })
})

test('the flux found for an opening gives that opening back', (assert) => {
  ;[[0.5, 8, 0], [0.1, 8, 1], [0.9, 4, 0.5], [0.02, 16, 0]]
    .forEach(([open, push, iron]) => {
      const back = ironSlope(fluxAt(open, push, iron), push, iron)
      assert.true(Math.abs(back - open) < 1e-6, `${open}: ${back}`)
    })
  assert.is(fluxAt(1, 8, 0), 0)
})

test('the sides\' path is a first-order high-pass, mixed', (assert) => {
  const cases = [
    [100, 100, 1, -3.0103],
    [50, 100, 1, -6.9897],
    [100, 100, 0.5, -2.0412],
    [100, 100, 0, 0]
  ]
  cases.forEach(([hz, corner, mix, want]) => {
    const got = keptDb(hz, corner, mix)
    assert.true(Math.abs(got - want) < 1e-3, `${hz} ${mix}: ${got}`)
  })
})
