// Releases, one plugin at a time. A plugin's version is its crate's;
// a version is released under the tag PLUGIN-vVERSION.
//
//   npm run release -- pending       the plugins that ship and whose
//                                    version has no tag yet, as JSON
//   npm run release -- dist PLUGIN   target/dist/PLUGIN/: the zip
//                                    (VST3, CLAP, manual, demos),
//                                    SHA256SUMS and NOTES.md
//   npm run release -- publish       `dist` and a GitHub release for
//                                    each pending plugin, through the
//                                    gh CLI
//
// The release workflow runs `publish` whenever main changes.

import { execFileSync } from 'node:child_process'
import { createHash } from 'node:crypto'
import {
  cpSync, existsSync, mkdirSync, readFileSync, rmSync, writeFileSync
} from 'node:fs'
import { join } from 'node:path'
import { pathToFileURL } from 'node:url'
import { bundle } from './bundle.js'
import { buildPdf, checkDocs, sectionsOf } from './docs.js'
import {
  ROOT, manifest, pending, pluginDir, plugins, tagOf, tags, versionOf
} from './lib/plugins.js'

const fail = (code, message) => Object.assign(new Error(message), { code })

export const zipName = (plugin, version, platform = process.platform) => {
  const os = { win32: 'windows', darwin: 'macos' }[platform] ?? platform
  return `${plugin}-${version}-${os}-x64.zip`
}

// What the release says: what the plugin is, what is new about it,
// and how to install it.
export const notes = (name, readme, novelty, zip) => {
  const about = readme.split('\n')
    .filter((line) => line.trim() && !/^[#!]/.test(line))[0] ?? ''
  const fresh = sectionsOf(novelty ?? '')['What is new'] ?? ''
  return [
    `# ${name}`,
    '',
    about,
    ...(fresh ? ['', '## What is new about it', '', fresh] : []),
    '',
    '## Install',
    '',
    `Unzip \`${zip}\`. Copy \`${name}.vst3\` (the whole folder) to`,
    '`C:\\Program Files\\Common Files\\VST3`, or `' + name + '.clap` to',
    '`C:\\Program Files\\Common Files\\CLAP`, then rescan in your host.',
    'The manual is the PDF in the zip. Windows 11, x64.',
    ''
  ].join('\n')
}

const zip = (dir, file) => {
  rmSync(file, { force: true })
  // Windows ships bsdtar, which writes a zip by the file's extension.
  if (process.platform === 'win32') {
    execFileSync('tar', ['-a', '-cf', file, '-C', dir, '.'])
  } else {
    execFileSync('zip', ['-qr', file, '.'], { cwd: dir })
  }
}

export const dist = async (plugin) => {
  const problems = checkDocs(plugin)
  if (problems.length) throw fail('DOCS', problems.join('\n'))

  const dir = pluginDir(plugin)
  const version = versionOf(manifest(plugin))
  const out = join(ROOT, 'target', 'dist', plugin)
  const stage = join(out, 'stage')
  rmSync(out, { recursive: true, force: true })
  mkdirSync(stage, { recursive: true })

  const { out: bundled, name, paths } = bundle(plugin)
  cpSync(join(bundled, `${name}.vst3`), join(stage, `${name}.vst3`), {
    recursive: true
  })
  cpSync(join(bundled, paths.clap), join(stage, paths.clap))
  const manual = await buildPdf(plugin)
  cpSync(manual, join(stage, `${name} manual.pdf`))
  execFileSync(
    process.env.CARGO || 'cargo',
    ['run', '-q', '--release', '-p', plugin, '--example', 'demo', '--',
      join(stage, 'demos')],
    { cwd: ROOT, stdio: 'inherit' }
  )
  cpSync(join(ROOT, 'LICENSE'), join(stage, 'LICENSE.txt'))

  const file = join(out, zipName(plugin, version))
  zip(stage, file)
  const sum = createHash('sha256').update(readFileSync(file)).digest('hex')
  writeFileSync(join(out, 'SHA256SUMS'), `${sum}  ${zipName(plugin, version)}\n`)
  const read = (f) => existsSync(f) ? readFileSync(f, 'utf8') : null
  writeFileSync(join(out, 'NOTES.md'), notes(
    name,
    read(join(dir, 'README.md')) ?? '',
    read(join(dir, 'NOVELTY.md')),
    zipName(plugin, version)
  ))
  rmSync(stage, { recursive: true, force: true })
  return { out, file, tag: tagOf(plugin, version), name, version }
}

const waiting = () => {
  const names = plugins()
  const manifests = Object.fromEntries(names.map((n) => [n, manifest(n)]))
  return pending(names, manifests, tags())
}

// The gh CLI's arguments for a release of what `dist` made.
export const releaseArgs = ({ out, file, tag, name, version }) => [
  'release', 'create', tag,
  file, join(out, 'SHA256SUMS'),
  '--title', `${name} ${version}`,
  '--notes-file', join(out, 'NOTES.md')
]

const publish = async () => {
  const todo = waiting()
  if (!todo.length) console.log('release: nothing is waiting to be released')
  for (const { name } of todo) {
    const made = await dist(name)
    execFileSync('gh', releaseArgs(made), { cwd: ROOT, stdio: 'inherit' })
    console.log(`release: published ${made.tag}`)
  }
  return 0
}

const main = async ([command, plugin]) => {
  try {
    if (command === 'pending') {
      console.log(JSON.stringify(waiting()))
      return 0
    }
    if (command === 'publish') return await publish()
    if (command === 'dist' && plugin) {
      const made = await dist(plugin)
      console.log(`release: ${made.file}`)
      console.log(`release: tag ${made.tag}`)
      return 0
    }
    console.error('release: npm run release -- pending | dist PLUGIN | publish')
    return 1
  } catch (err) {
    console.error(`release: ${err.message}${err.code ? ` (${err.code})` : ''}`)
    return 1
  }
}

if (import.meta.url === pathToFileURL(process.argv[1]).href) {
  process.exitCode = await main(process.argv.slice(2))
}
