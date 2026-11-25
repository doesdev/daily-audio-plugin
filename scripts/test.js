// The scripts' own tests: the parts that decide what gets made,
// checked and released.

import test from 'mvt'
import { mkdtempSync, readFileSync, rmSync, cpSync, existsSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import { layout, userFolders } from './bundle.js'
import {
  EM_DASH, EN_DASH, checkImages, checkNovelty, checkSettings, findDashes,
  imagesShown, sectionsOf
} from './docs.js'
import { browserPath, readActivePort } from './lib/browser.js'
import {
  KEBAB, ROOT, displayName, pascal, pending, ships, snake, tagOf, versionOf
} from './lib/plugins.js'
import { classId, create, rewrite } from './new.js'
import { notes, releaseArgs, zipName } from './release.js'

test('finds every em and en dash by line', (assert) => {
  const text = `fine - line\nan em${EM_DASH}dash\n\n2${EN_DASH}3 and ${EM_DASH}`

  assert.deepEqual(findDashes('a.md', text), [
    'a.md:2 has an em dash',
    'a.md:4 has an em dash',
    'a.md:4 has an en dash'
  ])
  assert.deepEqual(findDashes('a.md', 'plain - text'), [])
})

test('a manual, its pictures and its scenes must agree', (assert) => {
  const manual = '![One](images/one.png)\n\n![Two](images/two-b.png)\n' +
    '![One again](images/one.png) and [a link](images/not-shown.png)'

  assert.deepEqual(imagesShown(manual), ['one', 'two-b'])
  assert.deepEqual(checkImages(['one'], ['one'], ['one']), [])
  assert.deepEqual(checkImages(['one', 'gone'], ['one', 'spare'], ['one', 'new']), [
    'the manual shows images/gone.png, which is not there',
    'images/spare.png is in no part of the manual',
    'images/spare.png has no scene that makes it',
    'scene new has no picture yet: npm run shots'
  ])
})

test('the manual must name every setting as a host does', (assert) => {
  const manual = '**Drive** pushes it. Tone is mentioned, not described.'

  assert.deepEqual(checkSettings(['Drive'], manual), [])
  assert.deepEqual(checkSettings(['Drive', 'Tone'], manual), [
    'the manual does not describe the setting **Tone**'
  ])
})

test('a plugin that ships says what is new about it', (assert) => {
  const full = '# X\n\n## What is new\n\nIt folds time.\n\n' +
    '## Nearest existing things\n\n- A: differs.\n\n## Where I looked\n\nKVR.\n'
  const thin = '# X\n\n## What is new\n\nTODO: say\n\n## Where I looked\n\nKVR.'

  assert.deepEqual(sectionsOf(full)['What is new'], 'It folds time.')
  assert.deepEqual(checkNovelty(full), [])
  assert.deepEqual(checkNovelty(null), ['there is no NOVELTY.md'])
  assert.deepEqual(checkNovelty(thin), [
    'NOVELTY.md says nothing under "What is new"',
    'NOVELTY.md says nothing under "Nearest existing things"'
  ])
})

test('a manifest says its version and whether it ships', (assert) => {
  const shipped = '[package]\nname = "a"\nversion = "1.2.3"\n\n[lib]\n'
  const held = `${shipped}\n[package.metadata.kit]\nrelease = false\n`

  assert.is(versionOf(shipped), '1.2.3')
  assert.true(ships(shipped))
  assert.false(ships(held))
  assert.true(ships(`${shipped}\n[package.metadata.kit]\nrelease = true\n`))
  assert.false(ships(manifestOf('template')))
})

const manifestOf = (plugin) =>
  readFileSync(join(ROOT, 'plugins', plugin, 'Cargo.toml'), 'utf8')

test('only shipped plugins with an untagged version are pending', (assert) => {
  const toml = (version, extra = '') =>
    `[package]\nname = "x"\nversion = "${version}"\n${extra}`
  const manifests = {
    fresh: toml('0.1.0'),
    done: toml('0.1.0'),
    bumped: toml('0.2.0'),
    held: toml('0.1.0', '[package.metadata.kit]\nrelease = false\n')
  }
  const tagged = ['done-v0.1.0', 'bumped-v0.1.0']

  assert.deepEqual(pending(Object.keys(manifests), manifests, tagged), [
    { name: 'fresh', version: '0.1.0' },
    { name: 'bumped', version: '0.2.0' }
  ])
  assert.is(tagOf('tape-bloom', '0.1.0'), 'tape-bloom-v0.1.0')
})

test('names: kebab for folders, snake and Pascal for code', (assert) => {
  const good = ['bloom', 'tape-bloom', 'x2', 'a-1-b']
  const bad = ['', 'Tape', 'tape_bloom', '-tape', 'tape-', 'tape--bloom', '2x']

  good.forEach((name) => assert.true(KEBAB.test(name)))
  bad.forEach((name) => assert.false(KEBAB.test(name)))
  assert.is(snake('tape-bloom'), 'tape_bloom')
  assert.is(pascal('tape-bloom'), 'TapeBloom')
  assert.is(displayName('  name: "Tape Bloom",\n  vendor: "x"'), 'Tape Bloom')
})

test('a class id is four words as the spec writes them', (assert) => {
  const bytes = Buffer.from('9c1e52a74b3d4f108e6a2d915c07b3f4', 'hex')

  assert.is(classId(bytes),
    '[0x9C1E_52A7, 0x4B3D_4F10, 0x8E6A_2D91, 0x5C07_B3F4]')
  assert.not(classId(), classId())
})

test('a new plugin is the template under its own names', (assert) => {
  const about = {
    name: 'tape-bloom',
    display: 'Tape Bloom',
    instrument: false,
    ids: ['[1, 2, 3, 4]', '[5, 6, 7, 8]']
  }
  const lib = readFileSync(join(ROOT, 'plugins/template/src/lib.rs'), 'utf8')
  const made = rewrite(lib, about)

  assert.contains(made, 'name: "Tape Bloom"')
  assert.contains(made, 'clap_id: "dev.doesdev.tape-bloom"')
  assert.contains(made, 'vst3_processor: [1, 2, 3, 4]')
  assert.contains(made, 'vst3_controller: [5, 6, 7, 8]')
  assert.contains(made, 'pub struct TapeBloom')
  assert.contains(made, 'kit_clap::export!(TapeBloom);')
  assert.doesNotContain(made, 'Template')
  assert.doesNotContain(made, 'dev.doesdev.template')

  const toml = rewrite(manifestOf('template'), about)
  assert.contains(toml, 'name = "tape-bloom"')
  assert.true(ships(toml))
  const suite = rewrite('kit_test::suite!(template::Template);', about)
  assert.is(suite, 'kit_test::suite!(tape_bloom::TapeBloom);')

  const synth = rewrite(lib, { ...about, instrument: true })
  assert.contains(synth, 'role: Role::Instrument')
  assert.contains(synth, 'midi_in: true')
  assert.contains(synth, 'audio_in: 0')
})

test('new makes the folder, and refuses a bad or taken name', (assert) => {
  const root = mkdtempSync(join(tmpdir(), 'kit-new-'))
  try {
    cpSync(join(ROOT, 'plugins', 'template'), join(root, 'plugins', 'template'), {
      recursive: true
    })
    const dir = create('tape-bloom', 'Tape Bloom', { root })
    const novelty = readFileSync(join(dir, 'NOVELTY.md'), 'utf8')

    assert.true(existsSync(join(dir, 'src', 'lib.rs')))
    assert.false(existsSync(join(dir, 'docs', 'images')))
    assert.is(checkNovelty(novelty).length, 3)
    assert.throws(() => create('tape-bloom', 'Tape Bloom', { root }))
    assert.throws(() => create('Tape Bloom', 'Tape Bloom', { root }))
    assert.throws(() => create('other', '', { root }))
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
})

test('a bundle is laid out as hosts expect', (assert) => {
  const win = layout('Tape Bloom', 'win32').vst3.replace(/\\/g, '/')
  const linux = layout('Tape Bloom', 'linux').vst3.replace(/\\/g, '/')
  const folders = userFolders('win32', { LOCALAPPDATA: 'L' })

  assert.is(win, 'Tape Bloom.vst3/Contents/x86_64-win/Tape Bloom.vst3')
  assert.is(linux, 'Tape Bloom.vst3/Contents/x86_64-linux/Tape Bloom.so')
  assert.is(layout('Tape Bloom', 'win32').clap, 'Tape Bloom.clap')
  assert.is(folders.vst3.replace(/\\/g, '/'), 'L/Programs/Common/VST3')
})

test('release notes say what it is, what is new and how to install', (assert) => {
  const readme = '# Tape Bloom\n\n![x](y.png)\n\nA delay that blooms.\n\nMore.'
  const novelty = '## What is new\n\nIt blooms.\n\n## Where I looked\n\nKVR.'
  const made = notes('Tape Bloom', readme, novelty, 'tape-bloom-0.1.0.zip')

  assert.contains(made, '# Tape Bloom\n\nA delay that blooms.')
  assert.contains(made, '## What is new about it\n\nIt blooms.')
  assert.contains(made, 'Unzip `tape-bloom-0.1.0.zip`')
  assert.doesNotContain(notes('T', readme, null, 'z'), 'What is new')
  assert.is(zipName('tape-bloom', '0.1.0', 'win32'),
    'tape-bloom-0.1.0-windows-x64.zip')
  const args = releaseArgs({
    out: 'o', file: 'o/a.zip', tag: 'tape-bloom-v0.1.0', name: 'Tape Bloom',
    version: '0.1.0'
  })
  assert.deepEqual(args.slice(0, 4),
    ['release', 'create', 'tape-bloom-v0.1.0', 'o/a.zip'])
  assert.is(args[args.indexOf('--title') + 1], 'Tape Bloom 0.1.0')
})

test('the browser is found, and its DevTools port read', (assert) => {
  const only = (path) => (candidate) => candidate === path

  assert.is(browserPath({ BROWSER: '/my/chrome' }, () => false), '/my/chrome')
  assert.is(browserPath({}, only('/usr/bin/chromium')), '/usr/bin/chromium')
  assert.is(browserPath({}, () => false), null)
  assert.is(readActivePort('9222\n/devtools/browser/abc\n'),
    'ws://127.0.0.1:9222/devtools/browser/abc')
  assert.is(readActivePort('nonsense'), null)
})
