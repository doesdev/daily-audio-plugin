// `node scripts/benchmarks.js`: holds the kit and every plugin to the
// rule in docs/benchmarks.md. Each thing the kit does has an entry in
// kit/benchmarks.json naming who does it best, what they do, how the
// kit compares and the tests that prove it. Nothing is added to the
// kit without one, and "meets" is not claimed without a test.

import { readFileSync, existsSync } from 'node:fs'
import { join } from 'node:path'
import { pathToFileURL } from 'node:url'
import { ROOT, pluginDir, plugins } from './lib/plugins.js'

export const VERDICTS = ['exceeds', 'meets', 'under']

// The seven things docs/design.md says the best editors share.
export const POINTS = [
  'picture', 'precision', 'compare', 'explains', 'fits', 'character',
  'presets'
]

// Exports that are parts, not things a person does.
const PARTS = ['h', 'icon', 'ICONS', 'helpFor', 'auto', 'control']

// What the kit's page builds: every export after the pure parts.
export const builders = (source) => {
  const dom = source.slice(source.indexOf('// ---- the DOM'))
  return [...dom.matchAll(/^export const (\w+)/gm)]
    .map((match) => match[1])
    .filter((name) => !PARTS.includes(name))
}

// The names of the tests in a test file.
export const testNames = (source) =>
  [...source.matchAll(/^test\('((?:[^'\\]|\\.)*)'/gm)]
    .map((match) => match[1].replace(/\\'/g, "'"))

const filled = (text) => typeof text === 'string' && text.trim().length > 0

const leaderProblems = (where, leaders) => [
  ...(Array.isArray(leaders) && leaders.length
    ? []
    : [`${where}: names no leader`]),
  ...(leaders ?? [])
    .filter((leader) => !filled(leader.who) || !filled(leader.does))
    .map(() => `${where}: a leader needs who and what they do`)
]

// Everything wrong with the kit's register, in words.
export const registerProblems = (register, built, tests) => {
  const entries = register.entries ?? []
  const covered = entries.flatMap((entry) => entry.covers ?? [])
  const each = entries.flatMap((entry) => {
    const at = `kit: ${entry.id}`
    const proof = entry.proof ?? []
    const claims = entry.verdict !== 'under'
    return [
      ...(filled(entry.what) && filled(entry.ours)
        ? []
        : [`${at}: needs what it is and what ours does`]),
      ...(VERDICTS.includes(entry.verdict)
        ? []
        : [`${at}: verdict is not one of ${VERDICTS.join(', ')}`]),
      ...leaderProblems(at, entry.leaders),
      ...(claims && !proof.length
        ? [`${at}: claims "${entry.verdict}" with no test to prove it`]
        : []),
      ...proof.filter((name) => !tests.includes(name))
        .map((name) => `${at}: no test is named "${name}"`),
      ...(entry.verdict === 'under' && !(filled(entry.gap) && filled(entry.next))
        ? [`${at}: under the bar, so it must say the gap and what is next`]
        : [])
    ]
  })
  return [
    ...built.filter((name) => !covered.includes(name))
      .map((name) => `kit: ${name} is built but no benchmark covers it`),
    ...covered.filter((name) => !built.includes(name))
      .map((name) => `kit: a benchmark covers ${name}, which is not built`),
    ...each
  ]
}

// Everything wrong with one plugin's comparison with its field.
export const pluginProblems = (name, sheet) => {
  const at = `${name}`
  if (!sheet) return [`${at}: has no benchmark.json`]
  return [
    ...(sheet.plugin === name
      ? []
      : [`${at}: benchmark.json is about "${sheet.plugin}": write this one's`]),
    ...(filled(sheet.kind) ? [] : [`${at}: does not say what kind it is`]),
    ...leaderProblems(at, sheet.leaders),
    ...((sheet.leaders ?? []).length >= 2
      ? []
      : [`${at}: names fewer than two of the best of its kind`]),
    ...POINTS.flatMap((point) => {
      const said = sheet.points?.[point]
      if (!said) return [`${at}: says nothing on ${point}`]
      return [
        ...(VERDICTS.includes(said.verdict)
          ? []
          : [`${at}: ${point} has no verdict`]),
        ...(filled(said.note) ? [] : [`${at}: ${point} has no note`])
      ]
    })
  ]
}

const read = (file) => readFileSync(join(ROOT, file), 'utf8')

export const allProblems = () => {
  const register = JSON.parse(read('kit/benchmarks.json'))
  const tests = ['kit/ui/test.js', 'kit/ui/gallery.test.js']
    .flatMap((file) => testNames(read(file)))
  const sheets = plugins().flatMap((name) => {
    const file = join(pluginDir(name), 'benchmark.json')
    const sheet = existsSync(file) ? JSON.parse(readFileSync(file, 'utf8')) : null
    return pluginProblems(name, sheet)
  })
  return {
    register,
    problems: [
      ...registerProblems(register, builders(read('kit/ui/kit.js')), tests),
      ...sheets
    ]
  }
}

const main = () => {
  const { register, problems } = allProblems()
  problems.forEach((problem) => console.error(`benchmarks: ${problem}`))
  const count = (verdict) =>
    register.entries.filter((entry) => entry.verdict === verdict).length
  const unsourced = register.entries
    .flatMap((entry) => entry.leaders)
    .filter((leader) => !leader.source).length
  console.log(`benchmarks: ${register.entries.length} in the kit: ` +
    VERDICTS.map((verdict) => `${count(verdict)} ${verdict}`).join(', '))
  register.entries.filter((entry) => entry.verdict === 'under')
    .forEach((entry) => console.log(`benchmarks: under: ${entry.id}: ${entry.gap}`))
  if (unsourced) {
    console.log(`benchmarks: ${unsourced} claims about leaders rest on memory, ` +
      'not a source: check each when its control is next touched')
  }
  return problems.length ? 1 : 0
}

if (import.meta.url === pathToFileURL(process.argv[1]).href) {
  process.exitCode = main()
}
