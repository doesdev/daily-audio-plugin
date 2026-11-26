// The plugins in this repo and what their manifests say of them.

import { execFileSync } from 'node:child_process'
import { existsSync, readFileSync, readdirSync } from 'node:fs'
import { join } from 'node:path'

export const ROOT = join(import.meta.dirname, '..', '..')

export const KEBAB = /^[a-z][a-z0-9]*(-[a-z0-9]+)*$/

export const pluginDir = (plugin, root = ROOT) => join(root, 'plugins', plugin)

// Every folder under plugins/ that is a crate, by name.
export const plugins = (root = ROOT) => readdirSync(join(root, 'plugins'))
  .filter((name) => existsSync(join(pluginDir(name, root), 'Cargo.toml')))
  .sort()

// A plugin's page may carry its own tests, as `ui/test.js`, for what
// it works out itself (a curve that must agree with the sound's). The
// check runs each one found.
export const pageTests = (root = ROOT) => plugins(root)
  .map((name) => join(pluginDir(name, root), 'ui', 'test.js'))
  .filter((file) => existsSync(file))

export const manifest = (plugin, root = ROOT) =>
  readFileSync(join(pluginDir(plugin, root), 'Cargo.toml'), 'utf8')

export const versionOf = (cargoToml) =>
  cargoToml.match(/^version\s*=\s*"([^"]+)"/m)?.[1] ?? null

// Whether the plugin is one that ships: its Cargo.toml does not say
// `release = false` under [package.metadata.kit].
export const ships = (cargoToml) =>
  !/\[package\.metadata\.kit\][^[]*release\s*=\s*false/.test(cargoToml)

// The name a plugin shows a host, from its spec in src/lib.rs.
export const displayName = (source) =>
  source.match(/\bname:\s*"([^"]+)"/)?.[1] ?? null

// The tag a version of a plugin is released under.
export const tagOf = (plugin, version) => `${plugin}-v${version}`

export const tags = (root = ROOT) => execFileSync(
  'git', ['tag', '--list'], { cwd: root, encoding: 'utf8' }
).split('\n').filter(Boolean)

// The plugins that ship and whose current version has no tag yet.
export const pending = (names, manifests, tagged) => names
  .map((name) => ({ name, toml: manifests[name] }))
  .filter(({ toml }) => ships(toml))
  .map(({ name, toml }) => ({ name, version: versionOf(toml) }))
  .filter(({ name, version }) => !tagged.includes(tagOf(name, version)))

// The examples every plugin has: `spec`, which the docs tools run, and
// `demo`, which renders what a release ships to listen to. Cargo puts
// every package's examples in one folder, so each is named after its
// plugin: two plugins with an example of the same name overwrite each
// other's binary, and the wrong plugin's runs.
export const EXAMPLES = ['spec', 'demo']

export const exampleOf = (plugin, kind) => `${plugin}-${kind}`

// The example names a manifest declares.
export const examplesIn = (cargoToml) =>
  [...cargoToml.matchAll(/\[\[example\]\][^[]*?^name\s*=\s*"([^"]+)"/gm)]
    .map((match) => match[1])

// What is wrong with the examples across plugins, in words: a plugin
// missing its own, or a name declared by more than one.
export const exampleProblems = (manifests) => {
  const named = Object.entries(manifests)
    .flatMap(([plugin, toml]) => examplesIn(toml)
      .map((example) => ({ plugin, example })))
  const missing = Object.entries(manifests).flatMap(([plugin, toml]) =>
    EXAMPLES.map((kind) => exampleOf(plugin, kind))
      .filter((want) => !examplesIn(toml).includes(want))
      .map((want) => `${plugin}: declares no example "${want}"`))
  const shared = [...new Set(named.map(({ example }) => example))]
    .filter((example) =>
      named.filter((n) => n.example === example).length > 1)
    .map((example) => `example "${example}" is declared by more than ` +
      'one plugin')
  return [...missing, ...shared]
}

export const snake = (name) => name.replace(/-/g, '_')

export const pascal = (name) => name.split('-')
  .map((part) => part[0].toUpperCase() + part.slice(1))
  .join('')
