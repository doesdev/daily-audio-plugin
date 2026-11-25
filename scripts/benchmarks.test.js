// The benchmark rule's own teeth: each way of falling short is caught.

import test from 'mvt'
import {
  POINTS, builders, pluginProblems, registerProblems, testNames
} from './benchmarks.js'

const leader = { who: 'The best one', does: 'It well', source: null }
const good = {
  id: 'a',
  what: 'A thing',
  covers: ['value'],
  leaders: [leader],
  ours: 'The same',
  verdict: 'meets',
  proof: ['it works']
}
const problems = (entry, built = ['value'], tests = ['it works']) =>
  registerProblems({ entries: [entry] }, built, tests)

test('a sound register has nothing wrong with it', (assert) => {
  assert.deepEqual(problems(good), [])
})

test('each way a register falls short is said', (assert) => {
  const cases = [
    [{ ...good, proof: [] }, 'claims "meets" with no test'],
    [{ ...good, proof: ['it sings'] }, 'no test is named "it sings"'],
    [{ ...good, leaders: [] }, 'names no leader'],
    [{ ...good, leaders: [{ who: 'X' }] }, 'who and what they do'],
    [{ ...good, verdict: 'fine' }, 'verdict is not one of'],
    [{ ...good, verdict: 'under', proof: [] }, 'must say the gap'],
    [{ ...good, ours: '' }, 'what ours does'],
    [{ ...good, covers: [] }, 'value is built but no benchmark covers it'],
    [{ ...good, covers: ['value', 'dial'] }, 'covers dial, which is not built']
  ]

  cases.forEach(([entry, want]) => assert.true(
    problems(entry).some((said) => said.includes(want)), want
  ))
})

test('under the bar is allowed when it says the gap and what is next', (assert) => {
  const under = { ...good, verdict: 'under', proof: [], gap: 'No x', next: 'Do x' }

  assert.deepEqual(problems(under), [])
})

test('the kit\'s builders and its tests are read from their source', (assert) => {
  const source = `export const clamp = 1
// ---- the DOM
export const h = 1
export const value = 1
const hidden = 1
export const pad = 1`
  const tests = `test('a thing works', () => {})
test('it\\'s fine', async () => {})`

  assert.deepEqual(builders(source), ['value', 'pad'])
  assert.deepEqual(testNames(tests), ['a thing works', "it's fine"])
})

test('a plugin answers on all seven points against two leaders', (assert) => {
  const points = Object.fromEntries(
    POINTS.map((point) => [point, { verdict: 'meets', note: 'Yes' }]))
  const sheet = { plugin: 'echo', kind: 'Delay', leaders: [leader, leader], points }
  const says = (changed, want) => assert.true(
    pluginProblems('echo', changed).some((said) => said.includes(want)), want)

  assert.deepEqual(pluginProblems('echo', sheet), [])
  says(null, 'has no benchmark.json')
  says({ ...sheet, plugin: 'template' }, 'is about "template"')
  says({ ...sheet, leaders: [leader] }, 'fewer than two')
  says({ ...sheet, points: { ...points, picture: undefined } }, 'nothing on picture')
  says({ ...sheet, points: { ...points, presets: { verdict: 'under' } } },
    'presets has no note')
})
