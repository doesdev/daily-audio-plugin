import test from 'mvt'
import { permits, refused } from './licences.js'

test('permits permissive licence expressions', (assert) => {
  const cases = [
    ['MIT', true],
    ['MIT OR Apache-2.0', true],
    ['MIT/Apache-2.0', true],
    ['(MIT OR Apache-2.0) AND Unicode-3.0', true],
    ['Apache-2.0 WITH LLVM-exception', true],
    ['GPL-3.0-only OR MIT', true],
    ['GPL-3.0-only', false],
    ['LGPL-2.1-or-later', false],
    ['MIT AND GPL-2.0-only', false],
    ['AGPL-3.0', false],
    ['(MIT OR GPL-2.0', false],
    ['', false],
    [undefined, false]
  ]

  for (const [expr, want] of cases) assert.is(permits(expr), want, expr)
})

test('refused names the crate and its licence', (assert) => {
  const packages = [
    { name: 'ok', version: '1.0.0', license: 'MIT' },
    { name: 'gpl', version: '2.0.0', license: 'GPL-3.0' },
    { name: 'bare', version: '0.1.0', license: null }
  ]

  assert.is(JSON.stringify(refused(packages)), JSON.stringify([
    'gpl 2.0.0: GPL-3.0',
    'bare 0.1.0: no licence'
  ]))
})
