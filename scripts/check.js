// `npm run check`: everything a change must pass, in order, stopping
// at the first step that fails. CI runs exactly this, on Windows.
//
// The editor's WebView2 tests only build and run on Windows; elsewhere
// they are skipped and the check says so, because a pass here is then
// not the whole story.

import { spawnSync } from 'node:child_process'
import { mkdtempSync, rmSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import { pathToFileURL } from 'node:url'
import { checkDocs } from './docs.js'
import { ROOT, pageTests, plugins } from './lib/plugins.js'
import { takeShots } from './shots.js'

const WINDOWS = process.platform === 'win32'
const cargo = process.env.CARGO || 'cargo'

const run = (command, args) => () => {
  const done = spawnSync(command, args, { cwd: ROOT, stdio: 'inherit' })
  return done.status === 0
}

const node = (file) => run(process.execPath, [file])

// Each plugin's manual against the plugin itself.
const docs = () => plugins().map((plugin) => {
  const problems = checkDocs(plugin)
  problems.forEach((p) => console.error(`docs: ${plugin}: ${p}`))
  return problems.length === 0
}).every(Boolean)

// Each plugin's own page tests, where it has them.
const pages = () => pageTests()
  .map((file) => node(file)())
  .every(Boolean)

// Each plugin's every scene loads and draws with no error from its
// page. The pictures go to a temp folder: the ones in the repo are
// only replaced by `npm run shots`.
const scenes = async () => {
  const out = mkdtempSync(join(tmpdir(), 'kit-check-shots-'))
  try {
    for (const plugin of plugins()) {
      const taken = await takeShots(plugin, [], join(out, plugin))
      console.log(`scenes: ${plugin}: ${taken.length} drawn`)
    }
    return true
  } catch (err) {
    console.error(`scenes: ${err.message}${err.code ? ` (${err.code})` : ''}`)
    return false
  } finally {
    rmSync(out, { recursive: true, force: true })
  }
}

export const STEPS = [
  ['format', run(cargo, ['fmt', '--all', '--check'])],
  ['lint', run(cargo, [
    'clippy', '--workspace', '--all-targets', '--', '-D', 'warnings'
  ])],
  ['tests', run(cargo, ['test', '--workspace'])],
  ...(WINDOWS
    ? [['editor in a real webview', run(cargo, [
        'test', '--workspace', '--', '--ignored', '--test-threads=1'
      ])]]
    : []),
  ['markdown', node('scripts/lib/markdown.test.js')],
  ['scripts', node('scripts/test.js')],
  ['licences', node('scripts/licences.test.js')],
  ['every licence allows shipping', node('scripts/licences.js')],
  ['the page', node('kit/ui/test.js')],
  ['the rest of the page, on the gallery', node('kit/ui/gallery.test.js')],
  ['each plugin\'s own page', pages],
  ['the benchmark rule', node('scripts/benchmarks.test.js')],
  ['held to the best of the field', node('scripts/benchmarks.js')],
  ['manuals', docs],
  ['scenes', scenes]
]

const main = async () => {
  for (const [name, step] of STEPS) {
    console.log(`\ncheck: ${name}`)
    if (!(await step())) {
      console.error(`\ncheck: failed at: ${name}`)
      return 1
    }
  }
  console.log('\ncheck: all passed')
  if (!WINDOWS) {
    console.log('check: the WebView2 editor tests were skipped: ' +
      'they run on Windows only, in CI')
  }
  return 0
}

if (import.meta.url === pathToFileURL(process.argv[1]).href) {
  process.exitCode = await main()
}
