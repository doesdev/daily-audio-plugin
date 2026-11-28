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

// The tools every effect ships with where they make sense, and the
// call in a plugin's source that gives it each.
// The last three are for a plugin that works on mid and side, models
// a valve, or models a transformer: one that does none of them says
// so in a few words.
export const TOOLS = {
  mix: 'kit::mix(',
  match: 'kit::level_match(',
  mid_side: 'kit::listen(',
  tube: 'kit::tube_bias(',
  transformer: 'kit::iron_drive('
}

// A plugin says of each tool either `true`, and then its source has
// it, or why it does not make sense here.
const toolProblems = (at, sheet, source) => Object.entries(TOOLS)
  .flatMap(([tool, call]) => {
    const said = sheet.tools?.[tool]
    if (said === true) {
      const has = source.includes(call) || source.includes(`"${tool}"`)
      return has ? [] : [`${at}: says it has ${tool}, but its source does not`]
    }
    return filled(said)
      ? []
      : [`${at}: tools.${tool} must be true, or say why it makes no sense here`]
  })

// Everything wrong with one plugin's comparison with its field.
export const pluginProblems = (name, sheet, source = '') => {
  const at = `${name}`
  if (!sheet) return [`${at}: has no benchmark.json`]
  return [
    ...toolProblems(at, sheet, source),
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

// The names of the tests in a Rust source file.
export const rustTestNames = (source) =>
  [...source.matchAll(/#\[test\]\s+(?:#\[[^\]]*\]\s+)*fn (\w+)/g)]
    .map((match) => match[1])

const read = (file) => readFileSync(join(ROOT, file), 'utf8')

// The kit's Rust tests a benchmark may name as proof.
const RUST = [
  'kit/core/src/runner.rs', 'kit/core/src/scope.rs',
  'kit/webui/src/presets.rs', 'kit/webui/src/feed.rs',
  'kit/vst3/src/connect.rs', 'kit/core/src/learn.rs', 'kit/core/src/state.rs',
  'kit/vst3/tests/host.rs', 'kit/clap/tests/host.rs',
  'kit/dsp/src/level_match.rs', 'kit/dsp/src/mid_side.rs',
  'kit/dsp/src/tube.rs', 'kit/dsp/src/transformer.rs',
  'kit/core/src/link.rs', 'kit/core/src/denormal.rs',
  'kit/test/tests/teeth.rs', 'kit/dsp/src/oversample.rs',
  'kit/core/src/defaults.rs'
]

export const allProblems = () => {
  const register = JSON.parse(read('kit/benchmarks.json'))
  const tests = [
    ...['kit/ui/test.js', 'kit/ui/gallery.test.js', 'scripts/benchmarks.test.js']
      .flatMap((file) => testNames(read(file))),
    ...RUST.flatMap((file) => rustTestNames(read(file)))
  ]
  const sheets = plugins().flatMap((name) => {
    const file = join(pluginDir(name), 'benchmark.json')
    const sheet = existsSync(file) ? JSON.parse(readFileSync(file, 'utf8')) : null
    const lib = join(pluginDir(name), 'src', 'lib.rs')
    const source = existsSync(lib) ? readFileSync(lib, 'utf8') : ''
    return pluginProblems(name, sheet, source)
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
