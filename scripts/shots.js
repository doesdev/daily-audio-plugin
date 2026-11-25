// `npm run shots -- PLUGIN [scene ...]`: a plugin's screenshots, from
// its page itself. The page is taken out of the plugin's own binary
// (its `spec` example), so what is shot is what a host would show, and
// opened in a headless browser over a stand-in host: a fake window.ipc
// that answers `ready` with the hello the code made for the scene's
// values. Each scene in plugins/PLUGIN/docs/scenes.json becomes
// docs/images/NAME.png at twice the window's pixels.
//
// A scene is { name, values, theme, steps, crop, width, height }:
// `values` sets settings and meters by key, in their own terms;
// `theme` is "dark" (the default) or "light"; `steps` are
// [verb, ...args] from STEPS below, run in order before the shot;
// `crop` names the element (or elements) to cut the shot down to.
//
// Motion is off for the shot, and a scene whose page throws or logs an
// error fails rather than being shot.

import { spawnSync } from 'node:child_process'
import {
  existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync
} from 'node:fs'
import { tmpdir } from 'node:os'
import { dirname, join } from 'node:path'
import { pathToFileURL } from 'node:url'
import { withBrowser } from './lib/browser.js'
import { ROOT, pluginDir } from './lib/plugins.js'

const PAD = 8

const fail = (code, message) => Object.assign(new Error(message), { code })

export const readScenes = (plugin, root = ROOT) => {
  const file = join(pluginDir(plugin, root), 'docs', 'scenes.json')
  if (!existsSync(file)) throw fail('NOSCENES', `${file} is not there`)
  return JSON.parse(readFileSync(file, 'utf8'))
}

// Cargo where rustup puts it when it is not on PATH.
const cargo = () => process.env.CARGO || 'cargo'

// The plugin's `spec` example: its stdout for `args`, given `input`.
const spec = (plugin, args, input = '') => {
  const run = spawnSync(
    cargo(),
    ['run', '-q', '-p', plugin, '--example', 'spec', '--', ...args],
    { cwd: ROOT, input, encoding: 'utf8', maxBuffer: 64 * 1024 * 1024 }
  )
  if (run.error?.code === 'ENOENT') {
    throw fail('NOCARGO', 'cargo was not found: put it on PATH or set CARGO')
  }
  if (run.status !== 0) {
    throw fail('SPEC', `${plugin}'s spec example failed:\n${run.stderr}`)
  }
  return run.stdout
}

// What the code says of the plugin: its hello, one per scene, the
// paths its page is made of and its window's size.
export const dump = (plugin, scenes) =>
  JSON.parse(spec(plugin, ['--scenes'], JSON.stringify(scenes)))

// The page as the plugin's binary serves it, written under `site`.
export const writeSite = (plugin, paths, site) => paths.forEach((path) => {
  const file = join(site, path)
  mkdirSync(dirname(file), { recursive: true })
  writeFileSync(file, spec(plugin, ['--asset', path]))
})

const STILL = `*, *::before, *::after {
  transition: none !important;
  animation: none !important;
  caret-color: transparent !important;
}`

// The host: answers `ready` with the scene's hello and keeps what the
// page posts, for tests to read. `find` is how steps and crops name an
// element: a selector, or { sel, text } for the one whose text starts
// with `text`.
const standIn = (hello, theme) => `
try { localStorage.setItem('kit-theme', ${JSON.stringify(theme)}) } catch {}
// Each scene starts at the plugin's own density, whatever the last chose.
try { localStorage.removeItem('kit-density') } catch {}
window.posted = []
window.find = (spec) => {
  if (typeof spec === 'string') return document.querySelector(spec)
  return [...document.querySelectorAll(spec.sel)]
    .find((el) => el.textContent.trim().startsWith(spec.text)) ?? null
}
window.ipc = { postMessage: (text) => {
  const msg = JSON.parse(text)
  window.posted.push(msg)
  if (msg.type === 'ready') window.kit.receive(${JSON.stringify(hello)})
} }
`

// The page for one scene, in the site; its file URL.
export const writeScene = (site, name, hello, theme = 'dark') => {
  const html = readFileSync(join(site, 'index.html'), 'utf8')
  const main = '<script type="module" src="main.js"></script>'
  if (!html.includes(main)) {
    throw fail('NOMAIN', 'index.html no longer loads main.js as expected')
  }

  writeFileSync(join(site, `standin-${name}.js`), standIn(hello, theme))
  const page = html
    .replace('</head>', `<style>${STILL}</style>\n</head>`)
    .replace(main, `<script src="standin-${name}.js"></script>\n${main}`)
  const file = join(site, `scene-${name}.html`)
  writeFileSync(file, page)
  return pathToFileURL(file).href
}

// ---- in the page

const frames = `new Promise((r) =>
  requestAnimationFrame(() => requestAnimationFrame(() => r(true))))`

const named = (spec) =>
  typeof spec === 'string' ? spec : `${spec.sel} "${spec.text}"`

const centre = (spec) => `(() => {
  const el = window.find(${JSON.stringify(spec)})
  if (!el) return null
  el.scrollIntoView({ block: 'nearest' })
  const r = el.getBoundingClientRect()
  return { x: r.left + r.width / 2, y: r.top + r.height / 2 }
})()`

const at = async (tab, spec) => {
  const point = await tab.evaluate(centre(spec))
  if (!point) throw fail('NOELEMENT', `nothing matches ${named(spec)}`)
  return point
}

const mouse = (tab, type, point, button = 'left') =>
  tab.send('Input.dispatchMouseEvent', {
    type, ...point, button, buttons: button === 'left' ? 1 : 0, clickCount: 1
  })

export const STEPS = {
  click: async (tab, [spec]) => {
    const point = await at(tab, spec)
    await mouse(tab, 'mouseMoved', point, 'none')
    await mouse(tab, 'mousePressed', point)
    await mouse(tab, 'mouseReleased', point)
  },
  // Press on an element, move `dy` pixels up, and let go.
  drag: async (tab, [spec, dy]) => {
    const point = await at(tab, spec)
    await mouse(tab, 'mouseMoved', point, 'none')
    await mouse(tab, 'mousePressed', point)
    await mouse(tab, 'mouseMoved', { x: point.x, y: point.y - dy })
    await mouse(tab, 'mouseReleased', { x: point.x, y: point.y - dy })
  },
  // Press on an element, move `dx` pixels to the right, and let go.
  slide: async (tab, [spec, dx]) => {
    const point = await at(tab, spec)
    await mouse(tab, 'mouseMoved', point, 'none')
    await mouse(tab, 'mousePressed', point)
    await mouse(tab, 'mouseMoved', { x: point.x + dx, y: point.y })
    await mouse(tab, 'mouseReleased', { x: point.x + dx, y: point.y })
  },
  // Press on an element, move `dx` pixels to the right, and keep hold:
  // the shot shows the control as it is while being dragged.
  press: async (tab, [spec, dx]) => {
    const point = await at(tab, spec)
    await mouse(tab, 'mouseMoved', point, 'none')
    await mouse(tab, 'mousePressed', point)
    await mouse(tab, 'mouseMoved', { x: point.x + dx, y: point.y })
  },
  // A right-click: what opens a control's menu.
  menu: async (tab, [spec]) => {
    const point = await at(tab, spec)
    await tab.evaluate(`window.find(${JSON.stringify(spec)}).dispatchEvent(
      new MouseEvent('contextmenu', {
        bubbles: true, cancelable: true, clientX: ${point.x}, clientY: ${point.y}
      })) ?? true`)
  },
  dblclick: (tab, [spec]) => tab.evaluate(
    `window.find(${JSON.stringify(spec)}).dispatchEvent(
      new MouseEvent('dblclick', { bubbles: true })) ?? true`
  ),
  // Types into whatever has the keyboard, then presses `then` if given.
  type: async (tab, [text, then]) => {
    await tab.evaluate('document.activeElement.select?.() ?? true')
    await tab.send('Input.insertText', { text })
    if (then) await STEPS.key(tab, [then])
  },
  hover: async (tab, [spec]) => {
    const point = await at(tab, spec)
    await mouse(tab, 'mouseMoved', { x: point.x - 2, y: point.y - 1 }, 'none')
    await mouse(tab, 'mouseMoved', point, 'none')
  },
  // The pointer moved off everything, so no hover lingers in the shot.
  away: (tab) => mouse(tab, 'mouseMoved', { x: 1, y: 1 }, 'none'),
  focus: (tab, [spec]) => tab.evaluate(
    `window.find(${JSON.stringify(spec)}).focus() ?? true`
  ),
  // A key by name; `ctrl` and `shift` held with it if asked.
  key: async (tab, [key, held = {}]) => {
    const modifiers = (held.ctrl ? 2 : 0) + (held.shift ? 8 : 0)
    const code = key.length === 1 ? `Key${key.toUpperCase()}` : key
    const windowsVirtualKeyCode = { Enter: 13, Escape: 27 }[key] ??
      (key.length === 1 ? key.toUpperCase().charCodeAt(0) : 0)
    const event = { key, code, modifiers, windowsVirtualKeyCode }
    await tab.send('Input.dispatchKeyEvent', { type: 'keyDown', ...event })
    await tab.send('Input.dispatchKeyEvent', { type: 'keyUp', ...event })
  },
  // What the host would say next: a state or meters message.
  tell: (tab, [msg]) => tab.evaluate(
    `window.kit.receive(${JSON.stringify(msg)}) ?? true`
  ),
  wait: (tab, [ms]) => new Promise((resolve) => setTimeout(resolve, ms))
}

// The box to capture: the union of what `crop` names, padded, inside
// the window; or the whole window.
const clipOf = (crop) => `(() => {
  const specs = ${JSON.stringify([crop].flat())}
  const boxes = specs.map((s) => window.find(s))
    .filter((el) => el && el.getClientRects().length)
    .map((el) => el.getBoundingClientRect())
  if (boxes.length !== specs.length) return null
  const left = Math.max(0, Math.min(...boxes.map((b) => b.left)) - ${PAD})
  const top = Math.max(0, Math.min(...boxes.map((b) => b.top)) - ${PAD})
  const right = Math.min(innerWidth,
    Math.max(...boxes.map((b) => b.right)) + ${PAD})
  const bottom = Math.min(innerHeight,
    Math.max(...boxes.map((b) => b.bottom)) + ${PAD})
  return { x: left, y: top, width: right - left, height: bottom - top,
    scale: 1 }
})()`

const waitFor = async (tab, expression, ms, what) => {
  const until = Date.now() + ms
  while (Date.now() < until) {
    if (await tab.evaluate(expression)) return
    await new Promise((resolve) => setTimeout(resolve, 50))
  }
  const said = tab.errors.length ? `; the page said: ${tab.errors[0]}` : ''
  throw fail('PAGETIMEOUT', `the page did not ${what} within ${ms} ms${said}`)
}

// A tab on a scene's page, built and settled, its steps run.
export const openScene = async (browser, url, scene, size) => {
  const [width, height] = [scene.width ?? size[0], scene.height ?? size[1]]
  const tab = await browser.open()
  await tab.send('Emulation.setDeviceMetricsOverride', {
    width, height, deviceScaleFactor: 2, mobile: false
  })
  await tab.send('Emulation.setEmulatedMedia', {
    features: [{ name: 'prefers-reduced-motion', value: 'reduce' }]
  })
  await tab.goto(url)
  const ready = 'document.getElementById("app")?.dataset.ready === "true"'
  await waitFor(tab, ready, 10000, 'build itself from its hello')
  await tab.evaluate(frames)
  for (const [name, ...args] of scene.steps ?? []) {
    if (!STEPS[name]) throw fail('NOSTEP', `${scene.name}: no step "${name}"`)
    await STEPS[name](tab, args)
    await tab.evaluate(frames)
  }
  return tab
}

const shoot = async (browser, url, scene, size) => {
  const tab = await openScene(browser, url, scene, size)
  try {
    await tab.evaluate('document.fonts.ready.then(() => true)')
    await tab.evaluate(frames)
    if (tab.errors.length) {
      throw fail('PAGEERROR', `${scene.name}: ${tab.errors.join('; ')}`)
    }

    const clip = scene.crop ? await tab.evaluate(clipOf(scene.crop)) : null
    if (scene.crop && !clip) {
      const what = [scene.crop].flat().map(named).join(', ')
      throw fail('NOCROP', `${scene.name}: nothing shown matches ${what}`)
    }
    const { data } = await tab.send('Page.captureScreenshot', {
      format: 'png', ...(clip ? { clip } : {})
    })
    return Buffer.from(data, 'base64')
  } finally {
    await tab.close()
  }
}

// Runs `fn` with the plugin's page on disk: ({ site, dumped }).
export const withSite = async (plugin, scenes, fn) => {
  const work = mkdtempSync(join(tmpdir(), `kit-shots-${plugin}-`))
  try {
    const dumped = dump(plugin, scenes)
    writeSite(plugin, dumped.paths, work)
    return await fn({ site: work, dumped })
  } finally {
    // SHOTS_KEEP keeps the stand-in pages, to open one by hand.
    if (process.env.SHOTS_KEEP) console.log(`shots: the pages are in ${work}`)
    else rmSync(work, { recursive: true, force: true })
  }
}

// Every scene of `plugin` (or those named) into `out`; their names.
export const takeShots = async (plugin, names = [], out = null) => {
  const scenes = readScenes(plugin)
  const unknown = names.filter((n) => !scenes.some((s) => s.name === n))
  if (unknown.length) {
    throw fail('NOSCENE', `${plugin} has no scene named ${unknown.join(', ')}`)
  }
  const chosen = scenes.filter((s) => !names.length || names.includes(s.name))
  const images = out ?? join(pluginDir(plugin), 'docs', 'images')
  mkdirSync(images, { recursive: true })

  return withSite(plugin, chosen, ({ site, dumped }) =>
    withBrowser(async (browser) => {
      const taken = []
      for (const scene of chosen) {
        const url = writeScene(
          site, scene.name, dumped.scenes[scene.name], scene.theme
        )
        const png = await shoot(browser, url, scene, dumped.size)
        writeFileSync(join(images, `${scene.name}.png`), png)
        taken.push(scene.name)
      }
      return taken
    }))
}

const main = async ([plugin, ...names]) => {
  if (!plugin) {
    console.error('shots: say which plugin: npm run shots -- PLUGIN [scene]')
    return 1
  }
  try {
    const taken = await takeShots(plugin, names, process.env.SHOTS_OUT)
    console.log(`shots: ${taken.length} of ${plugin}: ${taken.join(', ')}`)
    return 0
  } catch (err) {
    console.error(`shots: ${err.message}${err.code ? ` (${err.code})` : ''}`)
    return 1
  }
}

if (import.meta.url === pathToFileURL(process.argv[1]).href) {
  process.exitCode = await main(process.argv.slice(2))
}
