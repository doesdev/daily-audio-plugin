// `npm run new -- NAME "Display Name" [--instrument]`: a new plugin,
// copied from plugins/template with its own names and its own ids.
//
// NAME is kebab-case and becomes the folder, the crate and the release
// tag. The ids a host knows the plugin by (two VST3 class ids and the
// CLAP id) are made here once and are fixed for good.

import { randomBytes } from 'node:crypto'
import {
  cpSync, existsSync, readFileSync, readdirSync, rmSync, statSync,
  writeFileSync
} from 'node:fs'
import { join } from 'node:path'
import { pathToFileURL } from 'node:url'
import { KEBAB, ROOT, pascal, pluginDir, snake } from './lib/plugins.js'

const fail = (code, message) => Object.assign(new Error(message), { code })

const TEXT = ['.rs', '.toml', '.js', '.css', '.md', '.json', '.html']

// A VST3 class id as four 32 bit words, written the way the spec is.
export const classId = (bytes = randomBytes(16)) => {
  const hex = (i) => bytes.readUInt32BE(i * 4).toString(16)
    .toUpperCase().padStart(8, '0')
  const word = (i) => `0x${hex(i).slice(0, 4)}_${hex(i).slice(4)}`
  return `[${[0, 1, 2, 3].map(word).join(', ')}]`
}

// A plugin's changelog starts with what it is: the run fills in the
// line. The check holds the newest entry to the crate's version.
const CHANGELOG = (display) => `# ${display}: what changed

## 0.1.0 (${new Date().toISOString().slice(0, 10)})

- First release: SAY WHAT IT IS IN A LINE.
`

const NOVELTY = (display) => `# What is new about ${display}

## What is new

TODO: one paragraph. What does ${display} do that nothing found below
does? Name the one idea.

## Nearest existing things

TODO: the closest plugins, papers, patents or techniques found, each
with a link and one line on how ${display} differs from it.

## Where I looked

TODO: the searches run and the places checked, so the next person can
see what was and was not covered.

## What would change this

TODO: what, if it turned up, would mean this is not new after all.

## Against the best of its kind

TODO: the two or three best plugins of this kind, and for each of the
seven things in docs/design.md ("The bar") how this editor compares:
the picture, precision, compare and recover, explaining itself,
fitting the screen, character, presets. Say plainly where it is worse.
`

// The template's file `text`, rewritten for the new plugin.
export const rewrite = (text, { name, display, instrument, ids }) => {
  const [processor, controller] = ids
  return text
    .replace(/\[package\.metadata\.kit\]\nrelease = false\n\n/, '')
    .replace(/# Not shipped:[^\n]*\n# kit end to end\.\n/, '')
    .replace(/^name = "template"$/m, `name = "${name}"`)
    .replace(/vst3_processor: \[[^\]]*\]/, `vst3_processor: ${processor}`)
    .replace(/vst3_controller: \[[^\]]*\]/, `vst3_controller: ${controller}`)
    .replace(/dev\.doesdev\.template/g, `dev.doesdev.${name}`)
    .replace(/role: Role::Effect/, instrument
      ? 'role: Role::Instrument'
      : 'role: Role::Effect')
    .replace(/midi_in: false/, `midi_in: ${instrument}`)
    .replace(/audio_in: 2/, `audio_in: ${instrument ? 0 : 2}`)
    .replace(/\btemplate::/g, `${snake(name)}::`)
    .replace(/-p template\b/g, `-p ${name}`)
    .replace(/\btemplate-(spec|demo)\b/g, `${name}-$1`)
    .replace(/target\/demos\/template/g, `target/demos/${name}`)
    .replace(/name: "Template"/, `name: "${display.replace(/"/g, '')}"`)
    .replace(/\bTemplate\b/g, pascal(name))
}

const filesUnder = (dir) => readdirSync(dir).flatMap((entry) => {
  const path = join(dir, entry)
  return statSync(path).isDirectory() ? filesUnder(path) : [path]
})

export const create = (name, display, { instrument = false, root = ROOT } = {}) => {
  if (!KEBAB.test(name ?? '')) {
    throw fail('BADNAME', `"${name}" is not kebab-case, like tape-bloom`)
  }
  if (!display) throw fail('NODISPLAY', 'give the name a host shows, too')
  const dir = pluginDir(name, root)
  if (existsSync(dir)) throw fail('EXISTS', `plugins/${name} is already there`)

  cpSync(pluginDir('template', root), dir, { recursive: true })
  rmSync(join(dir, 'docs', 'images'), { recursive: true, force: true })
  const ids = [classId(), classId()]
  filesUnder(dir)
    .filter((file) => TEXT.some((ext) => file.endsWith(ext)))
    .forEach((file) => writeFileSync(file, rewrite(
      readFileSync(file, 'utf8'), { name, display, instrument, ids }
    )))
  writeFileSync(join(dir, 'NOVELTY.md'), NOVELTY(display))
  writeFileSync(join(dir, 'CHANGELOG.md'), CHANGELOG(display))
  return dir
}

const NEXT = (name) => `
Next, in this order:
  1. plugins/${name}/NOVELTY.md: what is new, and what you found.
  2. plugins/${name}/src/lib.rs: the spec (settings, meters, size).
  3. plugins/${name}/tests/sound.rs: what it promises, failing.
  4. src/lib.rs again: the sound, until the tests pass.
  5. ui/main.js and ui/style.css: the editor.
  6. docs/scenes.json and docs/manual.md, then: npm run shots -- ${name}
  7. npm run check
`

const main = (args) => {
  const [name, display] = args.filter((a) => !a.startsWith('--'))
  try {
    const dir = create(name, display, {
      instrument: args.includes('--instrument')
    })
    console.log(`new: ${dir}${NEXT(name)}`)
    return 0
  } catch (err) {
    console.error(`new: ${err.message}${err.code ? ` (${err.code})` : ''}`)
    console.error('new: npm run new -- NAME "Display Name" [--instrument]')
    return 1
  }
}

if (import.meta.url === pathToFileURL(process.argv[1]).href) {
  process.exitCode = main(process.argv.slice(2))
}
