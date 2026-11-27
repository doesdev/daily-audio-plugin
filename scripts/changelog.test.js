import test from 'mvt'
import {
  changes, entries, kitProblems, kitSince, pluginProblems, unwritten
} from './changelog.js'

const LOG = `# Echo

## 0.2.0 (2026-11-27)

- Its tail can be frozen.
- Bypass no longer clicks.

## 0.1.0 (2026-11-25)

- First release.
`

const KIT = `# The kit

## 2026-11-27

- Settings link across instances.

## 2026-11-26

- A sidechain input.

## 2026-11-25

- The kit.
`

test('a changelog is read an entry a heading', (assert) => {
  const [top, first] = entries(LOG)
  assert.deepEqual([top.version, top.date, top.items], ['0.2.0', '2026-11-27', 2])
  assert.is(first.body, '- First release.')
  assert.deepEqual(entries(KIT).map((entry) => [entry.version, entry.date]), [
    [null, '2026-11-27'], [null, '2026-11-26'], [null, '2026-11-25']
  ])
  assert.deepEqual(entries('# Nothing yet\n'), [])
})

test('a plugin\'s changelog must say what its version changed', (assert) => {
  const said = (text, version = '0.2.0') =>
    pluginProblems('echo', text, version)

  assert.deepEqual(said(LOG), [])
  const cases = [
    [null, '0.2.0', 'is missing'],
    ['# Echo\n', '0.2.0', 'has no entry'],
    [LOG, '0.3.0', 'the newest entry is 0.2.0, the crate is 0.3.0'],
    [LOG.replace(' (2026-11-27)', ''), '0.2.0', 'is not "VERSION (DATE)"'],
    [LOG.replace('2026-11-27', 'today'), '0.2.0', 'has no date'],
    [LOG.replace(/- Its tail.*\n- Bypass.*\n/, 'Things.\n'), '0.2.0',
      'lists nothing'],
    [LOG.replace('2026-11-25', '2026-11-28'), '0.2.0',
      'is newer than the entry above it']
  ]
  cases.forEach(([text, version, want]) => assert.true(
    said(text, version).some((problem) => problem.includes(want)), want))
})

test('the kit\'s changelog is an entry a day, newest first', (assert) => {
  assert.deepEqual(kitProblems(KIT), [])
  const cases = [
    [null, 'is missing'],
    [KIT.replace('## 2026-11-27', '## 0.2.0 (2026-11-27)'),
      'is not a date alone'],
    [KIT.replace('## 2026-11-25', '## 2026-11-29'), 'is newer than'],
    [KIT.replace('- The kit.', 'The kit.'), 'lists nothing']
  ]
  cases.forEach(([text, want]) => assert.true(
    kitProblems(text).some((problem) => problem.includes(want)), want))
})

test('code that changed without a line in its changelog is caught', (assert) => {
  const cases = [
    [['kit/core/src/link.rs'], ['the kit changed']],
    [['kit/core/src/link.rs', 'kit/CHANGELOG.md'], []],
    [['kit/ui/kit.js'], ['the kit changed']],
    // Tests, the gallery, pictures and manuals may move alone.
    [['kit/ui/gallery.test.js', 'kit/vst3/tests/host.rs',
      'kit/ui/gallery/main.js', 'plugins/echo/docs/manual.md',
      'plugins/echo/tests/sound.rs', 'docs/kit.md'], []],
    [['plugins/echo/src/lib.rs', 'plugins/tide/ui/main.js',
      'plugins/tide/CHANGELOG.md'], ['echo changed']],
    [[], []]
  ]
  cases.forEach(([changed, want]) => {
    const got = unwritten(changed)
    assert.is(got.length, want.length, changed.join(' '))
    want.forEach((text, i) => assert.true(got[i].includes(text), text))
  })
})

test('a release says what changed, and what the kit under it gained since', (assert) => {
  const made = changes(LOG, KIT, '0.2.0')
  assert.true(made.startsWith('## What changed in 0.2.0\n\n- Its tail'))
  assert.true(made.includes('### In the kit under it'))
  // After the version before, up to this one's day.
  assert.true(made.includes('Settings link across instances.'))
  assert.true(made.includes('A sidechain input.'))
  assert.false(made.includes('- The kit.'))

  // A first release has everything the kit had by then.
  assert.deepEqual(kitSince(KIT, undefined, '2026-11-25')
    .map((entry) => entry.date), ['2026-11-25'])
  const first = changes(LOG, KIT, '0.1.0')
  assert.true(first.includes('- First release.'))
  assert.true(first.includes('- The kit.'))
  assert.false(first.includes('sidechain'))
  // A version the changelog does not have says nothing.
  assert.is(changes(LOG, KIT, '9.9.9'), '')
  assert.is(changes(null, null, '0.1.0'), '')
})
