// The kit's page: its pure parts here under node, then the page itself
// in a headless browser over a stand-in host, playing the template
// plugin. What the WebView2 tests check on Windows with a real host,
// this checks everywhere: a drag is one gesture, the host's word wins.

import test from 'mvt'
import {
  EMPTY, TICKS, angle, arc, clamp, describe, dragged, litTicks, nudged,
  otherTheme, reduce, snap, themeFor
} from './kit.js'
import { withBrowser } from '../../scripts/lib/browser.js'
import { openScene, withSite, writeScene } from '../../scripts/shots.js'

const range = {
  kind: 'range', lo: 0, hi: 500, step: 0, decimals: 0, unit: 'ms', skew: 1,
  steps: 0
}
const stepped = { ...range, step: 5, steps: 100 }
const list = { kind: 'list', choices: ['Soft', 'Hard', 'Fold'], steps: 2 }

test('values stay in range and land on their steps', (assert) => {
  const cases = [
    [range, 0.123, 0.123],
    [range, 1.7, 1],
    [range, -0.2, 0],
    [stepped, 0.123, 0.12],
    [list, 0.4, 0.5],
    [list, 0.8, 1]
  ]

  cases.forEach(([row, norm, want]) => assert.is(snap(row, norm), want))
  assert.is(clamp(5, 0, 3), 3)
})

test('a drag moves by its length, a fine one a tenth as far', (assert) => {
  assert.is(dragged(range, 0.5, 80), 1)
  assert.is(dragged(range, 0.5, -40), 0.25)
  assert.is(dragged(range, 0.5, 80, true), 0.55)
  assert.is(dragged(range, 0.9, 500), 1)
  // A list of three: a twitch does not change it, a pull does.
  assert.is(snap(list, dragged(list, 0, 10)), 0)
  assert.is(snap(list, dragged(list, 0, 70)), 0.5)
})

test('a nudge is a step, or a fiftieth where there are none', (assert) => {
  assert.is(nudged(list, 0, 1), 0.5)
  assert.is(nudged(list, 1, 1), 1)
  assert.is(nudged(stepped, 0.5, -1), 0.49)
  assert.is(nudged(range, 0.5, 1), 0.52)
  assert.is(nudged(range, 0.5, 1, true), 0.502)
})

test('a knob sweeps 270 degrees about straight up', (assert) => {
  assert.is(angle(0), -135)
  assert.is(angle(0.5), 0)
  assert.is(angle(1), 135)
  assert.is(arc(24, 24, 18, -135, 135),
    'M 11.27 36.73 A 18 18 0 1 1 36.73 36.73')
  assert.is(arc(24, 24, 18, 0, 90), 'M 24.00 6.00 A 18 18 0 0 1 42.00 24.00')
})

test('a knob lights its ticks from where it rests to its value', (assert) => {
  const last = TICKS - 1
  const middle = last / 2

  assert.is(litTicks(0), null)
  assert.deepEqual(litTicks(1), [0, last])
  assert.deepEqual(litTicks(0.5), [0, middle])
  assert.is(litTicks(0.5, true), null)
  assert.deepEqual(litTicks(0, true), [0, middle])
  assert.deepEqual(litTicks(1, true), [middle, last])
  assert.deepEqual(litTicks(7, true), [middle, last])
})

test('the theme is the one chosen before, else the system\'s', (assert) => {
  assert.is(themeFor('light', false), 'light')
  assert.is(themeFor('dark', true), 'dark')
  assert.is(themeFor(null, true), 'light')
  assert.is(themeFor(null, false), 'dark')
  assert.is(themeFor('sepia', false), 'dark')
  assert.is(otherTheme('light'), 'dark')
  assert.is(otherTheme('dark'), 'light')
})

test('the page describes a value until the host does', (assert) => {
  const hz = { ...range, lo: 200, hi: 16000, skew: 3, unit: 'Hz' }

  assert.is(describe(range, 0.24), '120 ms')
  assert.is(describe(stepped, 0.123), '60 ms')
  assert.is(describe(hz, 0.5), '2175 Hz')
  assert.is(describe({ ...range, unit: '', decimals: 2 }, 0.5), '250.00')
  assert.is(describe(list, 0.5), 'Hard')
  assert.is(describe({ kind: 'meter' }, 0.5), '')
})

test('the store folds what the host says', (assert) => {
  const hello = {
    type: 'hello',
    version: 1,
    name: 'Thing',
    build: '1.0.0',
    params: [range, list],
    meters: [{ kind: 'meter' }],
    values: [0.2, 0.5],
    texts: ['100 ms', 'Hard'],
    levels: [0],
    levelTexts: ['silent']
  }
  const ready = reduce(EMPTY, hello)
  const moved = reduce(ready, { type: 'local', index: 0, value: 0.4 })
  const told = reduce(moved, {
    type: 'state', values: [0.4, 1], texts: ['200 ms', 'Fold']
  })
  const metered = reduce(told, {
    type: 'meters', levels: [0.5], levelTexts: ['-30 dB']
  })

  assert.false(EMPTY.ready)
  assert.true(ready.ready)
  assert.is(ready.name, 'Thing')
  assert.deepEqual(moved.values, [0.4, 0.5])
  assert.deepEqual(moved.texts, ['200 ms', 'Hard'])
  assert.deepEqual(told.texts, ['200 ms', 'Fold'])
  assert.deepEqual(metered.levelTexts, ['-30 dB'])
  assert.is(reduce(ready, { type: 'unheard-of' }), ready)
})

// ---- the page itself

const PLUGIN = 'template'
const posted = (tab) => tab.evaluate('window.posted.slice(1)')
const text = (tab, sel) => tab.evaluate(
  `document.querySelector(${JSON.stringify(sel)})?.textContent ?? null`
)

// Runs `fn` with a tab on the template's page at its defaults.
const onPage = (steps, fn) => {
  const scene = { name: 'test', steps }
  return withSite(PLUGIN, [scene], ({ site, dumped }) =>
    withBrowser(async (browser) => {
      const url = writeScene(site, scene.name, dumped.scenes.test)
      const tab = await openScene(browser, url, scene, dumped.size)
      try {
        return await fn(tab)
      } finally {
        await tab.close()
      }
    }))
}

test('the page builds itself from the hello', async (assert) => {
  await onPage([], async (tab) => {
    assert.is(await text(tab, '.kit-name'), 'Template')
    assert.is(await tab.evaluate('document.title'), 'Template')
    assert.is(await tab.evaluate(
      'document.querySelectorAll(".kit-knob").length'
    ), 4)
    assert.is(await text(tab, '[data-key="drive"] .kit-value'), '12.0 dB')
    assert.is(await text(tab, '.kit-option[aria-pressed="true"]'), 'Soft')
    assert.deepEqual(await tab.evaluate('window.posted'), [{ type: 'ready' }])
    assert.deepEqual(tab.errors, [])
  })
})

test('a drag on a knob is one gesture for the host', async (assert) => {
  const steps = [['drag', '[data-key="mix"] .kit-knob-dial', -80]]
  await onPage(steps, async (tab) => {
    const sent = await posted(tab)

    assert.deepEqual(sent.map((m) => m.type), ['begin', 'set', 'end'])
    assert.true(sent.every((m) => m.id === 3))
    assert.is(sent[1].value, 0.5)
    // Shown at once, before the host has said what to call it.
    assert.is(await text(tab, '[data-key="mix"] .kit-value'), '50 %')
  })
})

test('a click, a key and a double-click are gestures too', async (assert) => {
  const steps = [
    ['click', { sel: '.kit-option', text: 'Fold' }],
    ['focus', '[data-key="drive"] .kit-knob-dial'],
    ['key', 'ArrowUp']
  ]
  await onPage(steps, async (tab) => {
    const sent = await posted(tab)
    const types = sent.map((m) => m.type).join(' ')

    assert.is(types, 'begin set end begin set end')
    assert.deepEqual([sent[1].id, sent[1].value], [1, 1])
    assert.is(sent[4].id, 0)
    assert.is(await text(tab, '.kit-option[aria-pressed="true"]'), 'Fold')

    await tab.evaluate(`document.querySelector('[data-key="drive"] svg')
      .dispatchEvent(new MouseEvent('dblclick', { bubbles: true })) ?? true`)
    const reset = (await posted(tab)).slice(6)
    assert.deepEqual(reset.map((m) => m.type), ['begin', 'set', 'end'])
    assert.is(await text(tab, '[data-key="drive"] .kit-value'), '12.0 dB')
  })
})

test('the theme switch flips the page and is remembered', async (assert) => {
  const theme = 'document.documentElement.dataset.theme'
  const ground = 'getComputedStyle(document.body).backgroundColor'
  await onPage([], async (tab) => {
    assert.is(await tab.evaluate(theme), 'dark')
    const dark = await tab.evaluate(ground)

    await tab.evaluate('document.querySelector(".kit-theme").click() ?? true')
    assert.is(await tab.evaluate(theme), 'light')
    assert.not(await tab.evaluate(ground), dark)
    assert.is(await tab.evaluate('localStorage.getItem("kit-theme")'), 'light')
    assert.is(
      await tab.evaluate('document.querySelector(".kit-theme").ariaLabel'),
      'Switch to the dark theme'
    )
    // The switch is the page's own business: the host hears nothing.
    assert.deepEqual(await posted(tab), [])
  })
})

test('the page carries its own typeface', async (assert) => {
  await onPage([], async (tab) => {
    assert.true(await tab.evaluate('document.fonts.check("13px Archivo")'))
    assert.contains(
      await tab.evaluate('getComputedStyle(document.body).fontFamily'),
      'Archivo'
    )
  })
})

test('what the host says replaces what the page shows', async (assert) => {
  const state = {
    type: 'state',
    values: [1, 0.5, 0, 0, 0.5],
    texts: ['36.0 dB', 'Hard', '200 Hz', '0 %', '-6.0 dB']
  }
  const meters = { type: 'meters', levels: [0.5], levelTexts: ['-30 dB'] }
  await onPage([['tell', state], ['tell', meters]], async (tab) => {
    assert.is(await text(tab, '[data-key="drive"] .kit-value'), '36.0 dB')
    assert.is(await text(tab, '.kit-option[aria-pressed="true"]'), 'Hard')
    assert.is(await text(tab, '[data-key="level"] .kit-value'), '-30 dB')
    assert.is(await tab.evaluate(`getComputedStyle(
      document.querySelector('.kit-meter-fill')).getPropertyValue('--level')`
    ), '0.5')
    // Automation is not the page's gesture: nothing goes back.
    assert.deepEqual(await posted(tab), [])
  })
})
