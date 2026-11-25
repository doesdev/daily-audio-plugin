// The kit's page: its pure parts here under node, then the page itself
// in a headless browser over a stand-in host, playing the template
// plugin. What the WebView2 tests check on Windows with a real host,
// this checks everywhere: a drag is one gesture, the host's word wins.

import test from 'mvt'
import {
  EMPTY, HOLD_MS, NO_HISTORY, angle, clamp, describe, differences, dragged,
  fitScale, held, normOf, nudged, otherTheme, parseValue, plainOf, recorded,
  redone, reduce, snap, themeFor, undone
} from './kit.js'
import { withBrowser } from '../../scripts/lib/browser.js'
import { openScene, withSite, writeScene } from '../../scripts/shots.js'

const range = {
  kind: 'range', lo: 0, hi: 500, step: 0, decimals: 0, unit: 'ms', skew: 1,
  steps: 0
}
const stepped = { ...range, step: 5, steps: 100 }
const hz = { ...range, lo: 200, hi: 16000, skew: 3, unit: 'Hz' }
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
  // Along a control 300 long, 150 is half its travel.
  assert.is(dragged(range, 0, 150, false, 300), 0.5)
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
})

test('a value in its own terms, and back', (assert) => {
  const cases = [[range, 0.24, 120], [stepped, 0.123, 60], [list, 0.5, 1]]

  cases.forEach(([row, norm, plain]) => assert.is(plainOf(row, norm), plain))
  assert.is(Math.round(plainOf(hz, 0.5)), 2175)
  assert.is(normOf(range, 250), 0.5)
  assert.is(normOf(list, 2), 1)
  assert.true(Math.abs(plainOf(hz, normOf(hz, 6000)) - 6000) < 1e-6)
})

test('the page describes a value until the host does', (assert) => {
  assert.is(describe(range, 0.24), '120 ms')
  assert.is(describe(stepped, 0.123), '60 ms')
  assert.is(describe(hz, 0.5), '2175 Hz')
  assert.is(describe({ ...range, unit: '', decimals: 2 }, 0.5), '250.00')
  assert.is(describe(list, 0.5), 'Hard')
  assert.is(describe({ kind: 'meter' }, 0.5), '')
})

test('a typed value is read the way a person writes one', (assert) => {
  const plain = (row, text) => {
    const norm = parseValue(row, text)
    return norm === null ? null : Math.round(plainOf(row, norm) * 100) / 100
  }
  const cases = [
    [range, '120', 120],
    [range, ' 120 ms ', 120],
    [range, '120MS', 120],
    [range, '12,5', 12.5],
    [range, '900', 500],
    [range, '-4', 0],
    [stepped, '62', 60],
    [hz, '8k', 8000],
    [hz, '1.2k Hz', 1200],
    [hz, '8000hz', 8000],
    [list, 'fold', 2],
    [list, 'ha', 1],
    [range, 'loud', null],
    [range, '', null],
    [range, '1.2.3', null],
    [list, 'warm', null],
    [{ kind: 'meter' }, '3', null]
  ]

  cases.forEach(([row, text, want]) => assert.is(plain(row, text), want))
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

test('the page scales whole to its window, in proportion', (assert) => {
  const base = [720, 400]

  assert.is(fitScale([720, 400], base), 1)
  assert.is(fitScale([1440, 800], base), 2)
  assert.is(fitScale([1440, 400], base), 1)
  assert.is(fitScale([1080, 900], base), 1.5)
  assert.is(fitScale([100, 100], base), 0.5)
})

test('history steps back through gestures and forward again', (assert) => {
  const a = { index: 0, before: 0.1, after: 0.4 }
  const b = { index: 2, before: 0, after: 1 }
  const one = recorded(NO_HISTORY, a)
  const two = recorded(one, b)
  const [back, undid] = undone(two)
  const [again, redid] = redone(back)

  assert.deepEqual(undid, b)
  assert.deepEqual(back, { past: [a], future: [b] })
  assert.deepEqual(redid, b)
  assert.deepEqual(again, two)
  assert.is(undone(NO_HISTORY), null)
  assert.is(redone(two), null)
  // A gesture that ended where it began is nothing to undo, and a new
  // one after stepping back forgets what was ahead.
  assert.is(recorded(two, { index: 1, before: 0.5, after: 0.5 }), two)
  assert.deepEqual(recorded(back, a).future, [])
})

test('compare moves only the settings that differ', (assert) => {
  assert.deepEqual(differences([0.1, 0.5, 1], [0.1, 0.7, 0]), [[1, 0.7], [2, 0]])
  assert.deepEqual(differences([0.1, 0.5], [0.1, 0.5]), [])
})

test('a meter holds its peak for a moment, then lets go', (assert) => {
  const start = { level: 0, at: 0 }
  const up = held(start, 0.8, 100)
  const still = held(up, 0.3, 100 + HOLD_MS - 1)
  const let_go = held(up, 0.3, 100 + HOLD_MS + 1)

  assert.deepEqual(up, { level: 0.8, at: 100 })
  assert.is(still, up)
  assert.deepEqual(let_go, { level: 0.3, at: 100 + HOLD_MS + 1 })
  assert.deepEqual(held(up, 0.9, 200), { level: 0.9, at: 200 })
})

test('the store folds what the host says', (assert) => {
  const hello = {
    type: 'hello',
    version: 1,
    name: 'Thing',
    build: '1.0.0',
    size: [600, 300],
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
  assert.deepEqual(ready.size, [600, 300])
  assert.deepEqual(moved.values, [0.4, 0.5])
  assert.deepEqual(moved.texts, ['200 ms', 'Hard'])
  assert.deepEqual(told.texts, ['200 ms', 'Fold'])
  assert.deepEqual(metered.levelTexts, ['-30 dB'])
  assert.is(reduce(ready, { type: 'unheard-of' }), ready)
})

// ---- the page itself

const PLUGIN = 'template'
const MIX = '[data-key="mix"]'
const DRIVE = '[data-key="drive"]'
const posted = (tab) => tab.evaluate('window.posted.slice(1)')
const text = (tab, sel) => tab.evaluate(
  `document.querySelector(${JSON.stringify(sel)})?.textContent ?? null`
)
const types = (sent) => sent.map((m) => m.type).join(' ')

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
    const count = (sel) => tab.evaluate(
      `document.querySelectorAll(${JSON.stringify(sel)}).length`
    )

    assert.is(await text(tab, '.kit-name'), 'Template')
    assert.is(await tab.evaluate('document.title'), 'Template')
    assert.is(await count('.kit-rule'), 3)
    assert.is(await count('.kit-knob'), 1)
    assert.is(await count('.kit-meter'), 2)
    assert.is(await count('.kit-screen canvas'), 1)
    assert.is(await text(tab, `${DRIVE} .kit-value`), '12.0 dB')
    assert.is(await text(tab, '.kit-option[aria-pressed="true"]'), 'Soft')
    assert.deepEqual(await tab.evaluate('window.posted'), [{ type: 'ready' }])
    assert.deepEqual(tab.errors, [])
  })
})

test('the screen is drawn, and drawn again when a setting moves', async (assert) => {
  const inked = `(() => {
    const c = document.querySelector('.kit-screen canvas')
    const d = c.getContext('2d').getImageData(0, 0, c.width, c.height).data
    let sum = 0
    for (let i = 3; i < d.length; i += 4) sum += d[i]
    return sum
  })()`
  const frame = 'new Promise((r) => requestAnimationFrame(() => r(true)))'
  await onPage([], async (tab) => {
    const before = await tab.evaluate(inked)
    assert.true(before > 0)

    await tab.evaluate(`document.querySelectorAll('.kit-option')[2].click() ?? true`)
    await tab.evaluate(frame)
    await tab.evaluate(frame)
    assert.not(await tab.evaluate(inked), before)
  })
})

test('a slide along a rule is one gesture for the host', async (assert) => {
  const width = `document.querySelector('${MIX} .kit-rule-slot')
    .getBoundingClientRect().width`
  await onPage([], async (tab) => {
    const half = Math.round(await tab.evaluate(width) / 2)
    await tab.evaluate(`window.find('${MIX} .kit-rule-slot').scrollIntoView() ?? true`)
    const { STEPS } = await import('../../scripts/shots.js')
    await STEPS.slide(tab, [`${MIX} .kit-rule-slot`, -half])
    const sent = await posted(tab)

    assert.is(types(sent), 'begin set end')
    assert.true(sent.every((m) => m.id === 3))
    assert.true(Math.abs(sent[1].value - 0.5) < 0.01)
    // Shown at once, before the host has said what to call it.
    assert.is(await text(tab, `${MIX} .kit-value`), '50 %')
  })
})

test('a click, a key and a double-click are gestures too', async (assert) => {
  const steps = [
    ['click', { sel: '.kit-option', text: 'Fold' }],
    ['focus', `${DRIVE} .kit-rule-slot`],
    ['key', 'ArrowUp'],
    ['dblclick', `${DRIVE} .kit-rule-slot`]
  ]
  await onPage(steps, async (tab) => {
    const sent = await posted(tab)

    assert.is(types(sent), 'begin set end begin set end begin set end')
    assert.deepEqual([sent[1].id, sent[1].value], [1, 1])
    assert.is(sent[4].id, 0)
    assert.is(await text(tab, '.kit-option[aria-pressed="true"]'), 'Fold')
    assert.is(await text(tab, `${DRIVE} .kit-value`), '12.0 dB')
  })
})

test('a value can be typed, and nonsense leaves it alone', async (assert) => {
  const steps = [
    ['dblclick', `${DRIVE} .kit-value`],
    ['type', '24', 'Enter'],
    ['dblclick', '[data-key="tone"] .kit-value'],
    ['type', '8k', 'Enter'],
    ['dblclick', `${MIX} .kit-value`],
    ['type', 'loud', 'Enter']
  ]
  await onPage(steps, async (tab) => {
    const sent = await posted(tab)

    assert.is(types(sent), 'begin set end begin set end')
    assert.is(await text(tab, `${DRIVE} .kit-value`), '24.0 dB')
    assert.is(await text(tab, '[data-key="tone"] .kit-value'), '8000 Hz')
    assert.is(await text(tab, `${MIX} .kit-value`), '100 %')
  })
})

test('undo and redo step through the page\'s own changes', async (assert) => {
  const steps = [
    ['click', { sel: '.kit-option', text: 'Hard' }],
    ['click', { sel: '.kit-option', text: 'Fold' }]
  ]
  const chosen = '.kit-option[aria-pressed="true"]'
  const disabled = (sel) => `document.querySelector('${sel}').disabled`
  await onPage(steps, async (tab) => {
    const { STEPS } = await import('../../scripts/shots.js')
    assert.true(await tab.evaluate(disabled('.kit-redo')))

    await STEPS.click(tab, ['.kit-undo'])
    assert.is(await text(tab, chosen), 'Hard')
    await STEPS.key(tab, ['z', { ctrl: true }])
    assert.is(await text(tab, chosen), 'Soft')
    assert.true(await tab.evaluate(disabled('.kit-undo')))
    await STEPS.key(tab, ['z', { ctrl: true, shift: true }])
    await STEPS.click(tab, ['.kit-redo'])
    assert.is(await text(tab, chosen), 'Fold')

    // Each step went to the host as a gesture of its own.
    const sent = (await posted(tab)).slice(6)
    assert.is(types(sent), Array(4).fill('begin set end').join(' '))
  })
})

test('compare flips between two sets of settings', async (assert) => {
  const chosen = '.kit-option[aria-pressed="true"]'
  const slot = '.kit-compare-slot[data-on="true"]'
  await onPage([], async (tab) => {
    const { STEPS } = await import('../../scripts/shots.js')
    assert.is(await text(tab, slot), 'A')

    // B starts as a copy of A: the first flip changes nothing.
    await STEPS.click(tab, ['.kit-compare'])
    assert.is(await text(tab, slot), 'B')
    assert.deepEqual(await posted(tab), [])

    await STEPS.click(tab, [{ sel: '.kit-option', text: 'Fold' }])
    await STEPS.click(tab, ['.kit-compare'])
    assert.is(await text(tab, slot), 'A')
    assert.is(await text(tab, chosen), 'Soft')
    await STEPS.click(tab, ['.kit-compare'])
    assert.is(await text(tab, chosen), 'Fold')
  })
})

test('the foot of the page says what the pointer is over', async (assert) => {
  const steps = [['hover', `${DRIVE} .kit-rule-slot`]]
  await onPage(steps, async (tab) => {
    const { STEPS } = await import('../../scripts/shots.js')
    assert.is(await text(tab, '.kit-hint-what'), 'Drive')
    assert.is(await text(tab, '.kit-hint-about'),
      'How hard the signal is pushed into the shaper')

    await STEPS.away(tab, [])
    assert.is(await text(tab, '.kit-hint-what'), '')
    assert.contains(await text(tab, '.kit-hint-about'), 'Drag to set')
  })
})

test('a clip lights a lamp that stays lit until clicked', async (assert) => {
  const hot = { type: 'meters', levels: [1, 0.5], levelTexts: ['0 dB', '-30 dB'] }
  const calm = { type: 'meters', levels: [0.5, 0.5], levelTexts: ['-30 dB', '-30 dB'] }
  const clipped = `document.querySelector('[data-key="level"]').dataset.clipped ?? null`
  await onPage([['tell', hot], ['tell', calm]], async (tab) => {
    assert.is(await tab.evaluate(clipped), 'true')
    assert.is(await text(tab, '[data-key="level"] .kit-value'), '-30 dB')
    assert.is(await tab.evaluate(
      `document.querySelector('[data-key="input"]').dataset.clipped ?? null`
    ), null)

    await tab.evaluate(`document.querySelector('[data-key="level"]').click() ?? true`)
    assert.is(await tab.evaluate(clipped), null)
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
  const meters = {
    type: 'meters', levels: [0.5, 0.25], levelTexts: ['-30 dB', '-45 dB']
  }
  await onPage([['tell', state], ['tell', meters]], async (tab) => {
    assert.is(await text(tab, `${DRIVE} .kit-value`), '36.0 dB')
    assert.is(await text(tab, '.kit-option[aria-pressed="true"]'), 'Hard')
    assert.is(await text(tab, '[data-key="level"] .kit-value'), '-30 dB')
    assert.is(await tab.evaluate(`getComputedStyle(document.querySelector(
      '[data-key="input"] .kit-meter-slot')).getPropertyValue('--level')`
    ), '0.25')
    // Automation is not the page's gesture: nothing goes back, and it
    // is not the page's to undo.
    assert.deepEqual(await posted(tab), [])
    assert.true(await tab.evaluate('document.querySelector(".kit-undo").disabled'))
  })
})
