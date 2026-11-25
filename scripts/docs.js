// `npm run docs -- PLUGIN [--pdf]`: checks a plugin's manual against
// the plugin itself and, with --pdf, prints it.
//
// A plugin's docs are plugins/PLUGIN/docs/manual.md, its pictures
// docs/images/NAME.png (each taken from a scene of that name in
// docs/scenes.json by `npm run shots`), and beside the manual a
// NOVELTY.md saying what is new about the plugin and what was found
// when looking for it elsewhere.
//
// The checks (also part of `npm run check`):
// - no em or en dash anywhere in the plugin's docs;
// - every picture the manual shows exists, every picture is shown,
//   and every picture has a scene that makes it;
// - the manual names every setting by the title a host shows;
// - a plugin that ships has a NOVELTY.md with something in each of
//   its sections.
//
// --pdf then writes target/dist/PLUGIN/PLUGIN-manual-VERSION.pdf.

import {
  existsSync, mkdirSync, readFileSync, readdirSync, writeFileSync
} from 'node:fs'
import { join } from 'node:path'
import { pathToFileURL } from 'node:url'
import { withBrowser } from './lib/browser.js'
import { escapeHtml, renderMarkdown } from './lib/markdown.js'
import { ROOT, pluginDir, ships, versionOf } from './lib/plugins.js'
import { dump, readScenes } from './shots.js'

export const EM_DASH = String.fromCodePoint(0x2014)
export const EN_DASH = String.fromCodePoint(0x2013)
const DASHES = { [EM_DASH]: 'an em dash', [EN_DASH]: 'an en dash' }

export const NOVELTY_SECTIONS = [
  'What is new',
  'Nearest existing things',
  'Where I looked',
  'Against the best of its kind'
]

const fail = (code, message) => Object.assign(new Error(message), { code })

// "file:line has an em dash" for each line with one.
export const findDashes = (name, text) => text.split('\n').flatMap(
  (line, i) => Object.keys(DASHES)
    .filter((dash) => line.includes(dash))
    .map((dash) => `${name}:${i + 1} has ${DASHES[dash]}`)
)

// The pictures a manual shows, by name without the extension.
export const imagesShown = (markdown) => [...new Set(
  [...markdown.matchAll(/!\[[^\]]*\]\(\s*images\/([a-z0-9-]+)\.png\s*\)/g)]
    .map((m) => m[1])
)]

export const checkImages = (shown, files, scenes) => [
  ...shown.filter((n) => !files.includes(n))
    .map((n) => `the manual shows images/${n}.png, which is not there`),
  ...files.filter((n) => !shown.includes(n))
    .map((n) => `images/${n}.png is in no part of the manual`),
  ...files.filter((n) => !scenes.includes(n))
    .map((n) => `images/${n}.png has no scene that makes it`),
  ...scenes.filter((n) => !files.includes(n))
    .map((n) => `scene ${n} has no picture yet: npm run shots`)
]

// Each setting must be named somewhere in the manual as a host names it.
export const checkSettings = (titles, markdown) => titles
  .filter((title) => !markdown.includes(`**${title}**`))
  .map((title) => `the manual does not describe the setting **${title}**`)

// Text under each heading of a NOVELTY.md, by heading.
export const sectionsOf = (markdown) => Object.fromEntries(
  markdown.split(/^## +/m).slice(1).map((part) => {
    const [heading, ...body] = part.split('\n')
    return [heading.trim(), body.join('\n').trim()]
  })
)

export const checkNovelty = (markdown) => {
  if (markdown === null) return ['there is no NOVELTY.md']
  const sections = sectionsOf(markdown)
  return NOVELTY_SECTIONS
    .filter((name) => !sections[name] || /^(TODO|TBD)\b/i.test(sections[name]))
    .map((name) => `NOVELTY.md says nothing under "${name}"`)
}

const read = (file) => existsSync(file) ? readFileSync(file, 'utf8') : null

// Everything wrong with the plugin's docs.
export const checkDocs = (plugin, hello = null) => {
  const dir = pluginDir(plugin)
  const docs = join(dir, 'docs')
  const manual = read(join(docs, 'manual.md'))
  if (manual === null) return [`${plugin} has no docs/manual.md`]

  const novelty = read(join(dir, 'NOVELTY.md'))
  const readme = read(join(dir, 'README.md'))
  const images = join(docs, 'images')
  const files = existsSync(images)
    ? readdirSync(images).filter((f) => f.endsWith('.png'))
      .map((f) => f.slice(0, -4))
    : []
  const scenes = readScenes(plugin).map((s) => s.name)
  const told = hello ?? dump(plugin, []).hello
  const cargo = readFileSync(join(dir, 'Cargo.toml'), 'utf8')

  return [
    ...findDashes('docs/manual.md', manual),
    ...findDashes('NOVELTY.md', novelty ?? ''),
    ...findDashes('README.md', readme ?? ''),
    ...(readme === null ? ['there is no README.md'] : []),
    ...checkImages(imagesShown(manual), files, scenes),
    ...checkSettings(told.params.map((p) => p.title), manual),
    ...(ships(cargo) ? checkNovelty(novelty) : [])
  ]
}

const CSS = `
@page { size: A4; margin: 22mm 20mm; }
body { font: 11pt/1.5 'Segoe UI', system-ui, sans-serif; color: #1b1d22; }
h1 { font-size: 26pt; margin: 0 0 4pt; }
h2 { font-size: 15pt; margin: 22pt 0 6pt; break-after: avoid; }
h3 { font-size: 12pt; margin: 16pt 0 4pt; break-after: avoid; }
p, li { orphans: 3; widows: 3; }
code { font: 9.5pt 'Cascadia Mono', Consolas, monospace; }
figure { margin: 14pt 0; break-inside: avoid; text-align: center; }
figure img { max-width: 100%; border-radius: 6pt; }
figcaption { font-size: 9pt; color: #5b6170; margin-top: 4pt; }
table { border-collapse: collapse; width: 100%; margin: 10pt 0; }
th, td { text-align: left; padding: 4pt 8pt; border-bottom: 0.5pt solid #c9cdd6; }
th { font-size: 9pt; text-transform: uppercase; letter-spacing: 0.06em; }
.edition { color: #5b6170; font-size: 9pt; margin-bottom: 18pt; }
.url { color: #5b6170; }
blockquote { margin: 10pt 0; padding: 6pt 12pt; background: #f1f3f7;
  border-radius: 4pt; }
`

// The manual as one HTML page, its pictures by file URL.
export const manualHtml = (plugin, markdown, version) => {
  const images = join(pluginDir(plugin), 'docs')
  const { html } = renderMarkdown(markdown, {
    imageSrc: (href) => pathToFileURL(join(images, href)).href,
    figureWidth: () => 520
  })
  const edition = `<div class="edition">${escapeHtml(plugin)} ` +
    `${escapeHtml(version)}</div>`
  const body = html.replace('</h1>', `</h1>\n${edition}`)
  return `<!doctype html>\n<html lang="en"><head><meta charset="utf-8">` +
    `<title>${escapeHtml(plugin)} manual</title><style>${CSS}</style>` +
    `</head><body>${body}</body></html>\n`
}

export const buildPdf = async (plugin) => {
  const dir = pluginDir(plugin)
  const manual = readFileSync(join(dir, 'docs', 'manual.md'), 'utf8')
  const version = versionOf(readFileSync(join(dir, 'Cargo.toml'), 'utf8'))
  const dist = join(ROOT, 'target', 'dist', plugin)
  mkdirSync(dist, { recursive: true })
  const page = join(dist, 'manual.html')
  writeFileSync(page, manualHtml(plugin, manual, version))

  const pdf = await withBrowser(async (browser) => {
    const tab = await browser.open()
    await tab.goto(pathToFileURL(page).href)
    return tab.pdf()
  })
  if (!pdf.subarray(0, 5).equals(Buffer.from('%PDF-'))) {
    throw fail('NOTPDF', 'the browser did not print a PDF')
  }
  const out = join(dist, `${plugin}-manual-${version}.pdf`)
  writeFileSync(out, pdf)
  return out
}

const main = async (args) => {
  const plugin = args.find((a) => !a.startsWith('--'))
  if (!plugin) {
    console.error('docs: say which plugin: npm run docs -- PLUGIN [--pdf]')
    return 1
  }
  try {
    const problems = checkDocs(plugin)
    problems.forEach((p) => console.error(`docs: ${plugin}: ${p}`))
    if (problems.length) return 1

    console.log(`docs: ${plugin}: the manual agrees with the plugin`)
    if (args.includes('--pdf')) console.log(`docs: ${await buildPdf(plugin)}`)
    return 0
  } catch (err) {
    console.error(`docs: ${err.message}${err.code ? ` (${err.code})` : ''}`)
    return 1
  }
}

if (import.meta.url === pathToFileURL(process.argv[1]).href) {
  process.exitCode = await main(process.argv.slice(2))
}
