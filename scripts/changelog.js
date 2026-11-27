// Changelogs: one for each plugin and one for the kit.
//
//   plugins/NAME/CHANGELOG.md   what changed for someone who uses the
//                               plugin, an entry a version:
//                                 ## 0.5.2 (2026-11-27)
//   kit/CHANGELOG.md            what changed in the kit, for whoever
//                               writes a plugin on it, an entry a day:
//                                 ## 2026-11-27
//
// Newest first, each entry a list. A release's notes take the
// plugin's entry for that version, and under it what changed in the
// kit since the plugin's version before.
//
//   node scripts/changelog.js            check them all, and that
//                                        what this branch changed is
//                                        written down
//   node scripts/changelog.js --base REF against REF, not origin/main

import { execFileSync } from 'node:child_process'
import { existsSync, readFileSync } from 'node:fs'
import { join } from 'node:path'
import { pathToFileURL } from 'node:url'
import {
  ROOT, manifest, pluginDir, plugins, ships, versionOf
} from './lib/plugins.js'

const DATE = /^\d{4}-\d{2}-\d{2}$/
const VERSION = /^\d+\.\d+\.\d+$/

// The entries of a changelog, in the order written. A heading is a
// version with its date in brackets, or for the kit a date alone.
export const entries = (text) => text.split(/^## /m).slice(1)
  .map((block) => {
    const [head, ...rest] = block.split('\n')
    const [, version, date] =
      head.trim().match(/^(?:(\S+) \((\S+)\)|(\S+))$/)?.slice(0, 3) ?? []
    const alone = head.trim().match(/^(\S+)$/)?.[1]
    const body = rest.join('\n').trim()
    return {
      head: head.trim(),
      version: version ?? null,
      date: date ?? alone ?? null,
      body,
      items: body.split('\n').filter((line) => /^- \S/.test(line)).length
    }
  })

const sound = (entry) => [
  !DATE.test(entry.date ?? '') && `"${entry.head}" has no date (YYYY-MM-DD)`,
  !entry.items && `"${entry.head}" lists nothing ("- " a change)`
].filter(Boolean)

const ordered = (list, key, what) => list.slice(1)
  .filter((entry, i) => key(entry) > key(list[i]))
  .map((entry) => `"${entry.head}" is ${what} than the entry above it`)

// What is wrong with a plugin's changelog, given its crate's version.
export const pluginProblems = (plugin, text, version) => {
  const file = `plugins/${plugin}/CHANGELOG.md`
  if (text == null) return [`${file} is missing`]
  const list = entries(text)
  if (!list.length) return [`${file} has no entry ("## 0.1.0 (DATE)")`]
  return [
    list[0].version !== version &&
      `the newest entry is ${list[0].version}, the crate is ${version}`,
    ...list.filter((entry) => !VERSION.test(entry.version ?? ''))
      .map((entry) => `"${entry.head}" is not "VERSION (DATE)"`),
    ...list.flatMap(sound),
    ...ordered(list, (entry) => entry.date ?? '', 'newer')
  ].filter(Boolean).map((problem) => `${file}: ${problem}`)
}

export const kitProblems = (text) => {
  const file = 'kit/CHANGELOG.md'
  if (text == null) return [`${file} is missing`]
  const list = entries(text)
  if (!list.length) return [`${file} has no entry ("## DATE")`]
  return [
    ...list.filter((entry) => entry.version)
      .map((entry) => `"${entry.head}" is not a date alone`),
    ...list.flatMap(sound),
    ...ordered(list, (entry) => entry.date ?? '', 'newer')
  ].map((problem) => `${file}: ${problem}`)
}

// What a branch changed that no changelog says: `changed` is every
// path it touched. Code is what counts: a plugin's src and ui, and
// the kit's crates and page. Pictures, manuals and tests may move
// without a line here.
export const unwritten = (changed) => {
  const touched = (file) => changed.includes(file)
  const code = (path) => /\/(src|ui)\//.test(path) &&
    !/(^|\/)(tests?|gallery)\/|\.test\.js$/.test(path)
  const names = [...new Set(changed
    .map((path) => path.match(/^plugins\/([^/]+)\//)?.[1])
    .filter(Boolean))]
  return [
    changed.some((path) => path.startsWith('kit/') && code(path)) &&
      !touched('kit/CHANGELOG.md') &&
      'the kit changed and kit/CHANGELOG.md does not say how',
    ...names
      .filter((name) => changed.some((path) =>
        path.startsWith(`plugins/${name}/`) && code(path)))
      .filter((name) => !touched(`plugins/${name}/CHANGELOG.md`))
      .map((name) =>
        `${name} changed and plugins/${name}/CHANGELOG.md does not say how`)
  ].filter(Boolean)
}

// The kit's entries a plugin's release should mention: those after
// the plugin's version before, up to this one's day.
export const kitSince = (kit, after, upTo) => entries(kit)
  .filter((entry) => DATE.test(entry.date ?? ''))
  .filter((entry) => (!after || entry.date > after) && entry.date <= upTo)

// What a release says changed: the plugin's entry for `version`, and
// what the kit under it gained since the version before. Empty for a
// version the changelog does not have.
export const changes = (text, kit, version) => {
  const list = entries(text ?? '')
  const at = list.findIndex((entry) => entry.version === version)
  if (at < 0) return ''
  const [entry, before] = [list[at], list[at + 1]]
  const under = kitSince(kit ?? '', before?.date, entry.date)
  return [
    `## What changed in ${version}`,
    '',
    entry.body,
    ...(under.length
      ? ['', '### In the kit under it', '',
          under.map((day) => day.body).join('\n')]
      : [])
  ].join('\n')
}

const read = (file) => existsSync(file) ? readFileSync(file, 'utf8') : null

export const check = (root = ROOT) => [
  ...kitProblems(read(join(root, 'kit', 'CHANGELOG.md'))),
  ...plugins(root)
    .filter((name) => ships(manifest(name, root)))
    .flatMap((name) => pluginProblems(
      name,
      read(join(pluginDir(name, root), 'CHANGELOG.md')),
      versionOf(manifest(name, root))))
]

// Every path this branch changed since it left `base`: nothing where
// there is no such place to compare with, as on main itself.
const changedSince = (base) => {
  const git = (...args) => execFileSync('git', args, {
    cwd: ROOT, encoding: 'utf8', stdio: ['ignore', 'pipe', 'ignore']
  }).trim()
  try {
    const from = git('merge-base', 'HEAD', base)
    return git('diff', '--name-only', from, 'HEAD').split('\n')
      .filter(Boolean)
  } catch {
    return []
  }
}

const main = (args) => {
  const base = args[args.indexOf('--base') + 1] ?? 'origin/main'
  const problems = [
    ...check(),
    ...unwritten(changedSince(args.includes('--base') ? base : 'origin/main'))
  ]
  problems.forEach((problem) => console.error(`changelog: ${problem}`))
  if (!problems.length) console.log('changelog: every change is written down')
  return problems.length ? 1 : 0
}

if (import.meta.url === pathToFileURL(process.argv[1]).href) {
  process.exitCode = main(process.argv.slice(2))
}
