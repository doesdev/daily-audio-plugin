// Hand-Me-Down's page works out a note's hit and its line of
// inheritance itself, to draw them. The hit is held to the same
// numbers as the sound's own `hit_weight` (tests/sound.rs): change one
// and change the other.

import test from 'mvt'
import {
  bodyAt, heightOf, heirs, hitWeight, RESTING, startsIn
} from './lineage.js'

test('the hit rises, holds and hands over as the sound has it', (assert) => {
  const cases = [
    // samples in, crossfade, hit, fade, weight
    [0, 48, 1440, 960, 0],
    [24, 48, 1440, 960, 0.5],
    [48, 48, 1440, 960, 1],
    [1439, 48, 1440, 960, 1],
    [1920, 48, 1440, 960, 0.5],
    [2400, 48, 1440, 960, 0],
    [9000, 48, 1440, 960, 0]
  ]

  cases.forEach(([i, cross, hit, fade, want]) => {
    const got = hitWeight(i, cross, hit, fade)
    assert.true(Math.abs(got - want) < 1e-3, `${i} in: ${got}, not ${want}`)
  })
})

test('a note starts where the age meter drops', (assert) => {
  const ages = [0.1, 0.2, 0.3, 0, 0.1, 0.2, 0.05, 0.15, 0.15, 1]

  assert.deepEqual(startsIn(ages), [3, 6])
  assert.deepEqual(startsIn([1, 1, 1]), [], 'a full meter is no note')
})

test('each body comes from the note back that far, if any', (assert) => {
  assert.deepEqual(heirs(4, 1), [null, 0, 1, 2])
  assert.deepEqual(heirs(4, 3), [null, null, null, 0])
})

test('a body is handed down while the earlier note rang', (assert) => {
  const first = RESTING.starts[1] - RESTING.starts[0]

  assert.false(bodyAt(RESTING, 0, 1, 0.1).handed, 'the first has none')
  assert.true(bodyAt(RESTING, 1, 1, first - 0.01).handed)
  assert.false(bodyAt(RESTING, 1, 1, first + 0.01).handed, 'then its own')
  assert.true(bodyAt(RESTING, 3, 2, 0.05).handed)
})

test('a level stands on a scale of decibels', (assert) => {
  const cases = [[0, 0], [0.01, 0], [1, 40 / 52], [Math.pow(10, 12 / 20), 1],
    [10, 1]]

  cases.forEach(([level, want]) => {
    const got = heightOf(level)
    assert.true(Math.abs(got - want) < 1e-6, `${level}: ${got}, not ${want}`)
  })
})
