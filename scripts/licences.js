// Fails when any Rust dependency's licence would bind a commercial
// release: every crate must be under licences from ALLOWED.

import { execFileSync } from 'node:child_process'
import { pathToFileURL } from 'node:url'

const ALLOWED = new Set([
  '0BSD',
  'Apache-2.0',
  'Apache-2.0 WITH LLVM-exception',
  'BSD-2-Clause',
  'BSD-3-Clause',
  'BSL-1.0',
  'CC0-1.0',
  'ISC',
  'MIT',
  'MIT-0',
  'MPL-2.0',
  'Unicode-3.0',
  'Unicode-DFS-2016',
  'Unlicense',
  'Zlib'
])

// SPDX expressions as cargo reports them, old `/` form included. OR
// binds looser than AND; parentheses group.
const tokenize = (expr) => expr
  .replace(/\//g, ' OR ')
  .replace(/([()])/g, ' $1 ')
  .split(/\s+/)
  .filter(Boolean)

export const permits = (expr) => {
  if (!expr) return false

  const tokens = tokenize(expr)
  let at = 0

  const term = () => {
    if (tokens[at] === '(') {
      at++
      const ok = any()
      if (tokens[at++] !== ')') throw new Error('unbalanced')
      return ok
    }

    let id = tokens[at++]
    if (tokens[at] === 'WITH') {
      id = `${id} WITH ${tokens[at + 1]}`
      at += 2
    }
    return ALLOWED.has(id) || ALLOWED.has(id.replace(/ WITH .*/, ''))
  }

  const all = () => {
    let ok = term()
    while (tokens[at] === 'AND') {
      at++
      ok = term() && ok
    }
    return ok
  }

  const any = () => {
    let ok = all()
    while (tokens[at] === 'OR') {
      at++
      ok = all() || ok
    }
    return ok
  }

  try {
    return any() && at === tokens.length
  } catch (ex) {
    return false
  }
}

export const refused = (packages) => packages
  .filter((p) => !permits(p.license))
  .map((p) => `${p.name} ${p.version}: ${p.license || 'no licence'}`)

const main = () => {
  const out = execFileSync('cargo', ['metadata', '--format-version', '1'], {
    encoding: 'utf8',
    maxBuffer: 64 * 1024 * 1024
  })
  const bad = refused(JSON.parse(out).packages)
  if (!bad.length) return console.log('licences: all permissive')

  console.error(`licences: not allowed for a commercial release:\n  ${bad.join('\n  ')}`)
  process.exitCode = 1
}

if (import.meta.url === pathToFileURL(process.argv[1]).href) main()
