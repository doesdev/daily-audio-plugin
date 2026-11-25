// `npm run gallery`: the kit's own pictures. The gallery
// (kit/ui/gallery) is the kit's page over a made-up plugin, with every
// control on it. Its scenes become docs/images/kit/NAME.png in both
// themes, so a change to the kit is looked at before it is committed.
// The kit's tests drive the same pages.

import {
  cpSync, mkdirSync, mkdtempSync, rmSync, writeFileSync
} from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import { pathToFileURL } from 'node:url'
import { withBrowser } from './lib/browser.js'
import { ROOT } from './lib/plugins.js'
import { openScene, writeScene } from './shots.js'

const UI = join(ROOT, 'kit', 'ui')
export const SIZE = [820, 500]

// A setting's row as the host would send it.
const range = (id, key, title, [lo, hi, rest], unit, about, more = {}) => ({
  id,
  key,
  title,
  short: title,
  about,
  group: '',
  kind: 'range',
  lo,
  hi,
  skew: 1,
  step: 0,
  steps: 0,
  decimals: 0,
  unit,
  default: (rest - lo) / (hi - lo),
  ...more
})

const list = (id, key, title, choices, about) => ({
  id,
  key,
  title,
  short: title,
  about,
  group: '',
  kind: 'list',
  choices,
  steps: choices.length - 1,
  default: 0
})

export const PARAMS = [
  range(0, 'drive', 'Drive', [0, 36, 12], 'dB',
    'How hard the signal is pushed into the shaper', { decimals: 1 }),
  range(1, 'tone', 'Tone', [200, 16000, 6000], 'Hz', 'Where the top rolls off'),
  range(2, 'mix', 'Mix', [0, 100, 100], '%', 'How much of it is the effect'),
  range(3, 'output', 'Output', [-24, 12, 0], 'dB',
    'Level after everything else', { decimals: 1 }),
  list(4, 'shape', 'Shape', ['Soft', 'Hard', 'Fold'], 'The bend'),
  list(5, 'bypass', 'Bypass', ['Off', 'On'], 'Passes the sound unchanged'),
  range(6, 'mid', 'Mid', [0, 100, 50], '%', 'How much of the middle'),
  range(7, 'side', 'Side', [0, 100, 50], '%', 'How much of the sides'),
  range(8, 'low', 'Low', [20, 2020, 220], 'Hz', 'Where the band starts'),
  range(9, 'high', 'High', [20, 2020, 1220], 'Hz', 'Where the band ends'),
  range(10, 'left', 'Attack, left', [0, 100, 12], 'ms', 'Left attack'),
  range(11, 'right', 'Attack, right', [0, 100, 14], 'ms', 'Right attack'),
  range(12, 'cutoff', 'Cutoff', [0, 100, 60], '%', 'Where the filter closes'),
  range(13, 'resonance', 'Resonance', [0, 100, 35], '%', 'The bump there'),
  range(14, 'send', 'Send', [-60, 0, -18], 'dB', 'How much goes to the echo')
]

const text = (row, norm) => row.kind === 'list'
  ? row.choices[Math.round(norm * row.steps)]
  : `${(row.lo + norm * (row.hi - row.lo)).toFixed(row.decimals)} ${row.unit}`

export const hello = (levels = [0.8, 0.6]) => ({
  type: 'hello',
  version: 1,
  name: 'Gallery',
  vendor: 'doesdev',
  build: '0.0.0',
  size: SIZE,
  params: PARAMS,
  meters: [
    { key: 'level', title: 'Level', short: 'Out', meter: 'level' },
    { key: 'input', title: 'Input', short: 'In', meter: 'level' }
  ],
  values: PARAMS.map((row) => row.default),
  texts: PARAMS.map((row) => text(row, row.default)),
  levels,
  levelTexts: levels.map((level) => `${Math.round(level * 60 - 60)} dB`)
})

// Runs `fn` with the gallery on disk as a plugin's page would be.
export const withGallery = async (fn) => {
  const site = mkdtempSync(join(tmpdir(), 'kit-gallery-'))
  try {
    mkdirSync(join(site, 'kit'))
    ;['kit.js', 'kit.css', 'fonts.css'].forEach((file) =>
      cpSync(join(UI, file), join(site, 'kit', file)))
    cpSync(join(UI, 'index.html'), join(site, 'index.html'))
    cpSync(join(UI, 'gallery', 'main.js'), join(site, 'main.js'))
    writeFileSync(join(site, 'style.css'), '')
    return await fn(site)
  } finally {
    rmSync(site, { recursive: true, force: true })
  }
}

// A tab on one page of the gallery, its steps run.
export const openGallery = (browser, site, scene) => {
  const url = writeScene(site, scene.name, hello(scene.levels), scene.theme)
  return openScene(browser, `${url}?page=${scene.page ?? 'main'}`, scene, SIZE)
}

const DRIVE = '[data-key="drive"]'

export const SCENES = [
  { name: 'main' },
  { name: 'presets', steps: [['click', '.kit-preset'], ['away']] },
  { name: 'settings', steps: [['click', '.kit-settings'], ['away']] },
  { name: 'menu', steps: [['menu', `${DRIVE} .kit-scrub`]] },
  {
    name: 'help',
    steps: [['click', '.kit-help'], ['hover', `${DRIVE} .kit-scrub`]]
  },
  { name: 'bypassed', steps: [['click', '.kit-power'], ['away']] },
  { name: 'groups', page: 'groups' },
  { name: 'pairs', page: 'pairs', levels: [0.92, 0.6] },
  {
    name: 'pairs-held',
    page: 'pairs',
    steps: [['press', '[data-key="mid"] .kit-oppose', 30]]
  }
]

const THEMES = ['light', 'dark']

export const takeGallery = (out) => withGallery((site) =>
  withBrowser(async (browser) => {
    mkdirSync(out, { recursive: true })
    const taken = []
    for (const scene of SCENES) {
      for (const theme of THEMES) {
        const name = `${scene.name}-${theme}`
        const tab = await openGallery(browser, site, { ...scene, name, theme })
        try {
          await tab.evaluate('document.fonts.ready.then(() => true)')
          if (tab.errors.length) throw new Error(`${name}: ${tab.errors[0]}`)
          const { data } = await tab.send('Page.captureScreenshot', {
            format: 'png'
          })
          writeFileSync(join(out, `${name}.png`), Buffer.from(data, 'base64'))
          taken.push(name)
        } finally {
          await tab.close()
        }
      }
    }
    return taken
  }))

if (import.meta.url === pathToFileURL(process.argv[1]).href) {
  const out = process.env.GALLERY_OUT ?? join(ROOT, 'docs', 'images', 'kit')
  const taken = await takeGallery(out)
  console.log(`gallery: ${taken.length} pictures in ${out}`)
}
