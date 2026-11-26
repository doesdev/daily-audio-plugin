// The kit's page: its pure parts here under node, then the page itself
// in a headless browser over a stand-in host, playing the template
// plugin. What the WebView2 tests check on Windows with a real host,
// this checks everywhere: a drag is one gesture, the host's word wins.

import test from 'mvt'
import {
  DENSITIES, EMPTY, HOLD_MS, NO_HISTORY, clamp, densityFor, describe,
  differences, dragged, fitScale, fitted, held, nextDensity, normOf, nudged,
  otherTheme, parseValue, parts, plainOf, recorded, redone, reduce, snap,
  spectrum, steady, themeFor, undone, zoneFills, zoneOf
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

test('a nudge is a step where they are few, else a fiftieth', (assert) => {
  assert.is(nudged(list, 0, 1), 0.5)
  assert.is(nudged(list, 1, 1), 1)
  // A hundred steps is a range that rounds: a fiftieth, or one step
  // when fine, where it used to crawl a step at a time.
  assert.is(nudged(stepped, 0.5, -1), 0.48)
  assert.is(nudged(stepped, 0.5, -1, true), 0.49)
  assert.is(nudged(range, 0.5, 1), 0.52)
  assert.is(nudged(range, 0.5, 1, true), 0.502)
})

test('a value splits into the number drawn large and its unit', (assert) => {
  const cases = [
    ['12.0 dB', ['12.0', 'dB']],
    ['-6.0 dB', ['-6.0', 'dB']],
    ['100 %', ['100', '%']],
    ['4.0 : 1', ['4.0', ': 1']],
    ['Soft', ['Soft', '']],
    ['', ['', '']]
  ]

  cases.forEach(([text, want]) => assert.deepEqual(parts(text), want))
})

test('a level fills six segments, breaking at -24, -12, -6, -3 and 0 dB', (assert) => {
  // A level is a fraction of the 60 dB under full scale.
  const at = (db) => 1 + db / 60
  const cases = [
    [-60, [0, 0, 0, 0, 0, 0]],
    [-42, [0.5, 0, 0, 0, 0, 0]],
    [-24, [1, 0, 0, 0, 0, 0]],
    [-18, [1, 0.5, 0, 0, 0, 0]],
    [-9, [1, 1, 0.5, 0, 0, 0]],
    [-4.5, [1, 1, 1, 0.5, 0, 0]],
    [-1.5, [1, 1, 1, 1, 0.5, 0]],
    [0, [1, 1, 1, 1, 1, 1]]
  ]

  cases.forEach(([db, want]) => assert.deepEqual(
    zoneFills(at(db)).map((v) => Math.round(v * 1000) / 1000), want, `${db} dB`
  ))
})

test('a level is named for the segment it has reached', (assert) => {
  const at = (db) => 1 + db / 60
  const cases = [
    [-60, 'ink'], [-30, 'ink'], [-20, 'green'], [-10, 'yellow'],
    [-5, 'orange'], [-2, 'red'], [0, 'red']
  ]

  cases.forEach(([db, want]) => assert.is(zoneOf(at(db)), want, `${db} dB`))
})

test('density is one of three, stepped round in order', (assert) => {
  assert.deepEqual(DENSITIES, ['roomy', 'regular', 'dense'])
  assert.is(densityFor('dense', 'roomy'), 'dense')
  assert.is(densityFor(null, 'regular'), 'regular')
  assert.is(densityFor('tiny', 'regular'), 'regular')
  assert.is(densityFor(null), 'roomy')
  assert.is(nextDensity('roomy'), 'regular')
  assert.is(nextDensity('regular'), 'dense')
  assert.is(nextDensity('dense'), 'roomy')
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
    assert.is(await count('.kit-glass'), 1)
    assert.is(await count('.kit-sheet'), 1)
    assert.is(await count('.kit-setting'), 4)
    assert.is(await count('.kit-meter'), 2)
    assert.is(await count('.kit-segment'), 12)
    assert.is(await count('.kit-picture canvas'), 1)
    assert.is(await text(tab, `${DRIVE} .kit-value`), '12.0 dB')
    assert.is(await text(tab, `${DRIVE} .kit-number`), '12.0')
    assert.is(await text(tab, '.kit-option[aria-pressed="true"]'), 'Soft')
    assert.deepEqual(await tab.evaluate('window.posted'), [{ type: 'ready' }])
    assert.deepEqual(tab.errors, [])
  })
})

test('nothing on the page runs past its edge', async (assert) => {
  const spills = `[...document.querySelectorAll('#app *')].filter((el) => {
    const out = document.getElementById('app').getBoundingClientRect()
    const r = el.getBoundingClientRect()
    return r.width && (r.right > out.right + 1 || r.bottom > out.bottom + 1)
  }).map((el) => el.className)`
  await onPage([], async (tab) => {
    assert.deepEqual(await tab.evaluate(spills), [])
  })
})

test('the picture is drawn, and drawn again when a setting moves', async (assert) => {
  const inked = `(() => {
    const c = document.querySelector('.kit-picture canvas')
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

test('a slide along a value is one gesture for the host', async (assert) => {
  const width = `document.querySelector('${MIX} .kit-scrub')
    .getBoundingClientRect().width`
  await onPage([], async (tab) => {
    const half = Math.round(await tab.evaluate(width) / 2)
    const { STEPS } = await import('../../scripts/shots.js')
    await STEPS.slide(tab, [`${MIX} .kit-scrub`, -half])
    const sent = await posted(tab)

    assert.is(types(sent), 'begin set end')
    assert.true(sent.every((m) => m.id === 3))
    assert.true(Math.abs(sent[1].value - 0.5) < 0.01)
    // Shown at once, before the host has said what to call it.
    assert.is(await text(tab, `${MIX} .kit-value`), '50 %')
    // Purple is for what is being touched, and it is let go of now.
    assert.is(await tab.evaluate(
      `document.querySelector('${MIX}').dataset.held ?? null`
    ), null)
  })
})

test('a click, a key and a double-click are gestures too', async (assert) => {
  const steps = [
    ['click', { sel: '.kit-option', text: 'Fold' }],
    ['focus', `${DRIVE} .kit-scrub`],
    ['key', 'ArrowUp'],
    ['dblclick', `${DRIVE} .kit-line`]
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

test('the foot of the sheet says what the pointer is over', async (assert) => {
  const steps = [['hover', `${DRIVE} .kit-scrub`]]
  await onPage(steps, async (tab) => {
    const { STEPS } = await import('../../scripts/shots.js')
    assert.is(await text(tab, '.kit-hint-what'), 'Drive')
    assert.is(await text(tab, '.kit-hint-about'),
      'How hard the signal is pushed into the shaper')

    await STEPS.away(tab, [])
    assert.is(await text(tab, '.kit-hint-what'), '')
    assert.contains(await text(tab, '.kit-hint-about'), 'Drag a number')
  })
})

test('a clip lights a lamp that stays lit until clicked', async (assert) => {
  const hot = { type: 'meters', levels: [1, 0.5], levelTexts: ['0 dB', '-30 dB'] }
  const calm = { type: 'meters', levels: [0.5, 0.5], levelTexts: ['-30 dB', '-30 dB'] }
  const clipped = `document.querySelector('[data-key="level"]').dataset.clipped ?? null`
  await onPage([['tell', hot], ['tell', calm]], async (tab) => {
    assert.is(await tab.evaluate(clipped), 'true')
    // The number is the peak that is held, not the reading after it.
    assert.is(await text(tab, '[data-key="level"] .kit-value'), '0 dB')
    assert.is(await tab.evaluate(
      `document.querySelector('[data-key="input"]').dataset.clipped ?? null`
    ), null)

    await tab.evaluate(`document.querySelector('[data-key="level"]').click() ?? true`)
    assert.is(await tab.evaluate(clipped), null)
    assert.is(await text(tab, '[data-key="level"] .kit-value'), '-30 dB')
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

test('density steps through three, shrinks the numbers and is remembered', async (assert) => {
  const density = 'document.documentElement.dataset.density'
  const size = `parseFloat(getComputedStyle(
    document.querySelector('${DRIVE} .kit-number')).fontSize)`
  await onPage([], async (tab) => {
    assert.is(await tab.evaluate(density), 'roomy')
    const roomy = await tab.evaluate(size)

    await tab.evaluate('document.querySelector(".kit-density").click() ?? true')
    assert.is(await tab.evaluate(density), 'regular')
    assert.true(await tab.evaluate(size) < roomy)
    assert.is(await tab.evaluate('localStorage.getItem("kit-density")'), 'regular')
    assert.is(
      await tab.evaluate('document.querySelector(".kit-density").ariaLabel'),
      'Density: regular. Switch to dense'
    )
    assert.deepEqual(await posted(tab), [])
  })
})

test('the page carries its own typeface', async (assert) => {
  await onPage([], async (tab) => {
    assert.true(await tab.evaluate(
      'document.fonts.check("13px \'Bricolage Grotesque\'")'
    ))
    assert.contains(
      await tab.evaluate('getComputedStyle(document.body).fontFamily'),
      'Bricolage Grotesque'
    )
  })
})

test('what the host says replaces what the page shows', async (assert) => {
  const state = {
    type: 'state',
    values: [1, 0.5, 0, 0, 0.5, 0],
    texts: ['36.0 dB', 'Hard', '200 Hz', '0 %', '-6.0 dB', 'Off']
  }
  const meters = {
    type: 'meters', levels: [0.5, 0.25], levelTexts: ['-30 dB', '-45 dB']
  }
  const fill = (n) => `getComputedStyle(document.querySelectorAll(
    '[data-key="input"] .kit-segment')[${n}]).getPropertyValue('--fill')`
  await onPage([['tell', state], ['tell', meters]], async (tab) => {
    assert.is(await text(tab, `${DRIVE} .kit-value`), '36.0 dB')
    assert.is(await text(tab, '.kit-option[aria-pressed="true"]'), 'Hard')
    assert.is(await text(tab, '[data-key="level"] .kit-value'), '-30 dB')
    // -45 dB is 15 of the first segment's 36 dB, and none of the next.
    assert.is(await tab.evaluate(fill(0)), '0.417')
    assert.is(await tab.evaluate(fill(1)), '0')
    // Automation is not the page's gesture: nothing goes back, and it
    // is not the page's to undo.
    assert.deepEqual(await posted(tab), [])
    assert.true(await tab.evaluate('document.querySelector(".kit-undo").disabled'))
  })
})

test('a setting held is purple, with its range, until it is let go', async (assert) => {
  const held = `document.querySelector('${DRIVE}').dataset.held ?? null`
  const shown = `getComputedStyle(
    document.querySelector('${DRIVE} .kit-range')).visibility`
  const colour = `getComputedStyle(
    document.querySelector('${DRIVE} .kit-number')).color`
  await onPage([], async (tab) => {
    const { STEPS } = await import('../../scripts/shots.js')
    const rest = await tab.evaluate(colour)
    assert.is(await tab.evaluate(shown), 'hidden')

    await STEPS.press(tab, [`${DRIVE} .kit-scrub`, 20])
    assert.is(await tab.evaluate(held), 'true')
    assert.is(await tab.evaluate(shown), 'visible')
    assert.not(await tab.evaluate(colour), rest)
    assert.is(await text(tab, `${DRIVE} .kit-range`), '0.0 dB36.0 dB')
    assert.is(await tab.evaluate('window.kit.touched()'), 'drive')
  })
})

test('a setting that rests inside its range fills from its default', async (assert) => {
  const fill = `document.querySelector('[data-key="output"] .kit-line-fill')
    .getBoundingClientRect().width`
  const state = {
    type: 'state',
    values: [0.33, 0, 0.7, 1, 0.75, 0],
    texts: ['12.0 dB', 'Soft', '6000 Hz', '100 %', '3.0 dB', 'Off']
  }
  await onPage([], async (tab) => {
    assert.is(await tab.evaluate(fill), 0)
    const { STEPS } = await import('../../scripts/shots.js')
    await STEPS.tell(tab, [state])
    assert.true(await tab.evaluate(fill) > 20)
  })
})

test('the picture is itself dragged: up is more Drive, as one gesture', async (assert) => {
  await onPage([], async (tab) => {
    const { STEPS } = await import('../../scripts/shots.js')
    await STEPS.drag(tab, ['.kit-picture', 40])
    const sent = await posted(tab)

    assert.is(types(sent), 'begin set end')
    assert.is(sent[1].id, 0)
    assert.true(sent[1].value > 1 / 3)
    assert.is(await tab.evaluate('window.kit.touched()'), null)

    await STEPS.dblclick(tab, ['.kit-picture'])
    assert.is(await text(tab, `${DRIVE} .kit-value`), '12.0 dB')
  })
})

test('the picture keeps the last few seconds of level, and draws them', async (assert) => {
  const corner = `(() => {
    const c = document.querySelector('.kit-picture canvas')
    const scale = c.width / c.clientWidth
    const [x, y] = [c.width - 3 * scale, c.height - 3 * scale]
    return c.getContext('2d').getImageData(x, y, 1, 1).data[3]
  })()`
  const frame = 'new Promise((r) => requestAnimationFrame(() => r(true)))'
  const reading = (level) => ['tell', {
    type: 'meters', levels: [level, 0.5], levelTexts: ['-6 dB', '-30 dB']
  }]
  await onPage([], async (tab) => {
    const { STEPS } = await import('../../scripts/shots.js')
    assert.is(await tab.evaluate(corner), 0)
    for (const level of [0.5, 0.7, 0.9]) await STEPS.tell(tab, reading(level).slice(1))
    await tab.evaluate(frame)
    await tab.evaluate(frame)

    assert.deepEqual(await tab.evaluate('window.kit.trail.map((l) => l[0])'),
      [0.5, 0.7, 0.9])
    // The newest reading is a column at the right, standing on the foot.
    assert.true(await tab.evaluate(corner) > 0)
  })
})

test('the plugin\'s bypass is in the header, not among the settings', async (assert) => {
  await onPage([], async (tab) => {
    assert.is(await tab.evaluate(
      'document.querySelectorAll(".kit-power").length'), 1)
    assert.is(await tab.evaluate(
      'document.querySelectorAll(".kit-sheet [data-key=bypass]").length'), 0)
  })
})

test('a spectrum finds a tone in its band, at its level', (assert) => {
  const size = 1024
  const tone = (band, level) => Array.from({ length: size }, (_, i) =>
    level * Math.sin(2 * Math.PI * band * i / size))
  const loudest = (bands) => bands.indexOf(Math.max(...bands))
  const cases = [[32, 1, 0], [100, 0.5, -6.02], [7, 0.1, -20]]

  cases.forEach(([band, level, db]) => {
    const bands = spectrum(tone(band, level))
    assert.is(bands.length, size / 2)
    assert.is(loudest(bands), band)
    assert.true(Math.abs(bands[band] - db) < 0.1, `${level} is ${bands[band]} dB`)
    // Well away from the tone there is nothing.
    assert.true(bands[(band + 200) % 500] < -100)
  })
  // Silence is a floor, not an error, and a ragged length is cut down.
  assert.true(spectrum(new Array(1000).fill(0)).every((v) => v === -180))
  assert.is(spectrum(new Array(1000).fill(0)).length, 256)
})

test('a wave is drawn from where it climbs through zero, so it stands still', (assert) => {
  const wave = Array.from({ length: 200 }, (_, i) => Math.sin((i - 30) / 10))

  assert.is(steady(wave, 100), 31)
  // Not from a crossing too late to leave a whole span after it.
  assert.is(steady(wave, 190), 0)
  assert.is(steady(new Array(50).fill(0.5), 10), 0)
})

test('the store keeps the sound the host sends, at full scale', (assert) => {
  const told = reduce(EMPTY, {
    type: 'signal', rate: 48000, scale: 16384, input: [8192, -16384], output: [0, 32768]
  })

  assert.is(EMPTY.signal, null)
  assert.deepEqual(told.signal, { rate: 48000, input: [0.5, -1], output: [0, 2] })
})

test('with audio running, the picture is the sound itself', async (assert) => {
  const row = `(() => {
    const c = document.querySelector('.kit-picture canvas')
    const d = c.getContext('2d').getImageData(0, 0, c.width, 6).data
    let sum = 0
    for (let i = 3; i < d.length; i += 4) sum += d[i]
    return sum
  })()`
  const frame = 'new Promise((r) => requestAnimationFrame(() => r(true)))'
  // A square wave at full scale: its top runs along the top of the
  // glass, where the model wave at its defaults never reaches.
  const square = Array.from({ length: 1024 }, (_, i) =>
    (Math.floor(i / 64) % 2 ? 16384 : -16384))
  const signal = {
    type: 'signal', rate: 48000, scale: 16384, input: square, output: square
  }
  await onPage([], async (tab) => {
    const { STEPS } = await import('../../scripts/shots.js')
    assert.is(await tab.evaluate(row), 0)

    await STEPS.tell(tab, [signal])
    await tab.evaluate(frame)
    await tab.evaluate(frame)
    assert.true(await tab.evaluate(row) > 0)
  })
})

test('a window of another shape is filled, not left with an empty band', (assert) => {
  const base = [820, 500]
  const cases = [
    // The design's own shape, and twice it: scaled, nothing to fill.
    [[820, 500], { zoom: 1, width: 820, height: 500 }],
    [[1640, 1000], { zoom: 2, width: 820, height: 500 }],
    // Taller than the design: no bigger, and the page is as tall.
    [[820, 900], { zoom: 1, width: 820, height: 900 }],
    // Wider: as tall as it can be, and the page is as wide.
    [[1640, 500], { zoom: 1, width: 1640, height: 500 }],
    // Smaller and squarer: shrunk to fit across, filled downwards.
    [[410, 500], { zoom: 0.5, width: 820, height: 1000 }]
  ]

  cases.forEach(([size, want]) => assert.deepEqual(fitted(size, base), want))
})

test('in a tall or a wide window the glass takes the room and nothing is cut', async (assert) => {
  const edges = `(() => {
    const sheet = document.querySelector('.kit-sheet').getBoundingClientRect()
    const glass = document.querySelector('.kit-glass').getBoundingClientRect()
    return {
      below: window.innerHeight - sheet.bottom,
      beside: window.innerWidth - glass.right,
      picture: document.querySelector('.kit-picture')
        .getBoundingClientRect().height
    }
  })()`
  const spills = `[...document.querySelectorAll('#app *')].filter((el) => {
    const r = el.getBoundingClientRect()
    return r.width && (r.right > innerWidth + 1 || r.bottom > innerHeight + 1)
  }).map((el) => el.className)`
  const shaped = (tab, width, height) => tab.send(
    'Emulation.setDeviceMetricsOverride',
    { width, height, deviceScaleFactor: 1, mobile: false })
  const frame = 'new Promise((r) => requestAnimationFrame(() => r(true)))'
  await onPage([], async (tab) => {
    const at = await tab.evaluate(edges)
    for (const [width, height] of [[820, 900], [1380, 500], [640, 700]]) {
      await shaped(tab, width, height)
      await tab.evaluate(frame)
      const now = await tab.evaluate(edges)
      const scale = Math.min(width / 820, height / 500)
      assert.true(now.below < 40 * scale, `${width}x${height} below: ${now.below}`)
      assert.true(now.beside < 40 * scale, `${width}x${height} beside: ${now.beside}`)
      assert.deepEqual(await tab.evaluate(spills), [])
    }
    await shaped(tab, 820, 900)
    await tab.evaluate(frame)
    assert.true((await tab.evaluate(edges)).picture > at.picture + 300)
  })
})

test('a frequency in whole hertz drags and nudges like a range, not a hertz at a time', (assert) => {
  // 30 to 1000 Hz, landing on whole hertz: 970 steps.
  const follow = { kind: 'range', lo: 30, hi: 1000, skew: 2.5, step: 1, steps: 970 }

  // The whole range in one span, as a setting with no steps has.
  assert.is(dragged(follow, 0, 160), 1)
  assert.is(dragged(follow, 0.25, 40), 0.5)
  // A notch of the wheel is a fiftieth of the range, rounded to a step.
  assert.is(nudged(follow, 0, 1), Math.round(0.02 * 970) / 970)
  assert.true(nudged(follow, 0.5, 1) - 0.5 > 0.019)
  // Fine is a five-hundredth, and never less than one step.
  assert.is(nudged(follow, 0, 1, true), Math.round(0.002 * 970) / 970)
  const coarse = { ...follow, steps: 100 }
  assert.is(nudged(coarse, 0, 1, true), 0.01)

  // A few steps are still felt one at a time.
  const count = { kind: 'range', steps: 8 }
  assert.is(nudged(count, 0, 1), 1 / 8)
  assert.is(dragged(count, 0, 24 * 8, false, 160), 1)
})
