// The rest of the kit's page, driven on the gallery: presets, a
// value's menu, the panels, help, bypass, groups, the fader, paired
// controls and the pad. Each is held to what docs/benchmarks.md says
// the best of its kind does.

import test from 'mvt'
import {
  EMPTY, NO_HISTORY, TIES, fallFor, fallen, fingerprint, helpFor, holdFor,
  ironCurve, prefsFor, textFit, tubeCurve,
  presetMoves, presetValues, recorded, redone, reduce, tagFor, undone
} from './kit.js'
import { withBrowser } from '../../scripts/lib/browser.js'
import { PARAMS, openGallery, withGallery } from '../../scripts/gallery.js'
import { STEPS } from '../../scripts/shots.js'

const [DRIVE, TONE, MIX, OUTPUT, SHAPE] = PARAMS

test('a preset names settings in their own terms and leaves the rest', (assert) => {
  const rows = [DRIVE, TONE, SHAPE]
  const values = presetValues(rows, { drive: 18, shape: 'Fold', gone: 3 })

  assert.deepEqual(values, [0.5, TONE.default, 1])
  assert.deepEqual(presetValues(rows), rows.map((row) => row.default))
  assert.deepEqual(presetValues(rows, { shape: 'Nope' }), rows.map((r) => r.default))
})

test('a preset is told apart by its first four numbers', (assert) => {
  const rows = [DRIVE, SHAPE, TONE, MIX, OUTPUT, DRIVE]

  assert.deepEqual(fingerprint(rows, [0.1, 1, 0.2, 0.3, 0.4, 0.5]),
    [0.1, 0.2, 0.3, 0.4])
})

test('a preset moves only what differs and is not locked', (assert) => {
  assert.deepEqual(presetMoves([0, 0.5, 1], [0.2, 0.5, 0]),
    [[0, 0, 0.2], [2, 1, 0]])
  assert.deepEqual(presetMoves([0, 0.5, 1], [0.2, 0.5, 0], [2]), [[0, 0, 0.2]])
})

test('a preset loaded is one step to undo', (assert) => {
  const batch = [{ index: 0, before: 0, after: 1 }, { index: 2, before: 1, after: 0 }]
  const loaded = recorded(NO_HISTORY, { batch })
  const [back, change] = undone(loaded)

  assert.is(loaded.past.length, 1)
  assert.deepEqual(change.batch, batch)
  assert.deepEqual(redone(back)[1].batch, batch)
  assert.is(recorded(NO_HISTORY, { batch: [] }), NO_HISTORY)
})

test('a pair moves together, apart, or one against the other', (assert) => {
  const start = { lead: 0.4, other: 0.5 }
  const cases = [
    ['linked', 0.6, 0.7], ['linked', 1, 1], ['mirrored', 0.6, 0.3],
    ['mirrored', 1, 0], ['opposed', 0.6, 0.4], ['opposed', 0, 1]
  ]

  cases.forEach(([tie, lead, want]) => assert.true(
    Math.abs(TIES[tie](start, lead) - want) < 1e-9, `${tie} at ${lead}`
  ))
})

test('preferences fall back to their defaults, one by one', (assert) => {
  assert.deepEqual(prefsFor(null),
    { hold: '2 s', motion: 'Full', fine: 'With Shift', fall: 'Medium' })
  assert.is(fallFor('Fast'), 40 / 60)
  assert.is(fallFor('Medium'), 20 / 60)
  assert.is(fallFor(undefined), 20 / 60)
  assert.is(prefsFor({ hold: 'Off', motion: 'Wobbly' }).hold, 'Off')
  assert.is(prefsFor({ hold: 'Off', motion: 'Wobbly' }).motion, 'Full')
  assert.is(holdFor('Off'), 0)
  assert.is(holdFor('2 s'), 2000)
  assert.is(holdFor('Until pressed'), Infinity)
})

test('help adds a setting\'s range and where it rests', (assert) => {
  assert.is(helpFor(DRIVE), ' From 0.0 dB to 36.0 dB. Rests at 12.0 dB.')
  assert.is(helpFor(SHAPE), ' One of Soft, Hard, Fold.')
  assert.is(helpFor(undefined), '')
})

// ---- the page

const key = (name) => `[data-key="${name}"]`
const text = (tab, sel) => tab.evaluate(
  `document.querySelector(${JSON.stringify(sel)})?.textContent ?? null`
)
const posted = (tab) => tab.evaluate('window.posted.slice(1)')
const types = (sent) => sent.map((m) => m.type).join(' ')
const click = (tab, spec) => STEPS.click(tab, [spec])
const has = (tab, sel) => tab.evaluate(
  `Boolean(document.querySelector(${JSON.stringify(sel)}))`
)

const onGallery = (page, steps, fn) => withGallery((site) =>
  withBrowser(async (browser) => {
    const tab = await openGallery(browser, site, { name: 'test', page, steps })
    try {
      await fn(tab)
      if (tab.errors.length) throw new Error(tab.errors.join('; '))
    } finally {
      await tab.close()
    }
  }))

const spills = `[...document.querySelectorAll('#app *')].filter((el) => {
  const out = document.getElementById('app').getBoundingClientRect()
  const r = el.getBoundingClientRect()
  return r.width && (r.right > out.right + 1 || r.bottom > out.bottom + 1)
}).map((el) => el.className)`

test('a preset loads as one step, names itself, and undoes as one', async (assert) => {
  const steps = [['click', '.kit-preset']]
  await onGallery('main', steps, async (tab) => {
    assert.is(await text(tab, '.kit-preset-at'), 'Init')
    await tab.evaluate(`[...document.querySelectorAll('.kit-preset-pick')]
      .find((el) => el.textContent.includes('Broken radio')).click() ?? true`)

    assert.is(await text(tab, '.kit-preset-at'), 'Broken radio')
    await click(tab, '.kit-done')
    assert.is(await text(tab, `${key('drive')} .kit-value`), '34.0 dB')
    assert.is(await text(tab, '.kit-option[aria-pressed="true"]'), 'Fold')
    assert.is(await tab.evaluate(
      'document.querySelector(".kit-preset").dataset.changed'), 'false')

    await STEPS.key(tab, ['z', { ctrl: true }])
    assert.is(await text(tab, `${key('drive')} .kit-value`), '12.0 dB')
    assert.is(await text(tab, '.kit-option[aria-pressed="true"]'), 'Soft')
    assert.is(await tab.evaluate(
      'document.querySelector(".kit-preset").dataset.changed'), 'true')
  })
})

test('pointing at a preset tries it, and leaving puts things back', async (assert) => {
  const drive = 'window.kit.store.get().values[0]'
  await onGallery('main', [['click', '.kit-preset']], async (tab) => {
    const before = await tab.evaluate(drive)
    await tab.evaluate(`[...document.querySelectorAll('.kit-preset-pick')]
      .find((el) => el.textContent.includes('Warm tape'))
      .dispatchEvent(new PointerEvent('pointermove', { movementX: 1 })) ?? true`)
    assert.is(await tab.evaluate(drive), 0.5)

    await tab.evaluate(`document.querySelector('.kit-preset-list')
      .dispatchEvent(new PointerEvent('pointerleave')) ?? true`)
    assert.is(await tab.evaluate(drive), before)
    // Trying one is not a change of the person's: nothing to undo.
    assert.true(await tab.evaluate('document.querySelector(".kit-undo").disabled'))
  })
})

test('a preset of one\'s own is saved, kept, found and deleted', async (assert) => {
  const kept = 'JSON.parse(localStorage.getItem("kit-presets:Gallery"))'
  const steps = [
    ['dblclick', `${key('drive')} .kit-value`], ['type', '30', 'Enter'],
    ['click', '.kit-preset'], ['click', '.kit-save']
  ]
  await onGallery('main', steps, async (tab) => {
    await tab.send('Input.insertText', { text: 'Init' })
    await STEPS.key(tab, ['Enter'])

    // A factory name is not written over.
    assert.is(await text(tab, '.kit-preset-at'), 'Init (mine)')
    const saved = await tab.evaluate(kept)
    assert.is(saved.length, 1)
    assert.true(Math.abs(saved[0].values.drive - 30 / 36) < 1e-9)

    await click(tab, '.kit-find')
    await tab.send('Input.insertText', { text: 'mine' })
    assert.is(await tab.evaluate(
      'document.querySelectorAll(".kit-preset-row").length'), 1)
    await click(tab, '.kit-preset-drop')
    assert.deepEqual(await tab.evaluate(kept), [])
    assert.is(await text(tab, '.kit-preset-at'), 'Init')
  })
})

test('a value\'s menu resets, copies, pastes and locks', async (assert) => {
  const item = (name) => ({ sel: '.kit-pop button', text: name })
  const steps = [
    ['dblclick', `${key('drive')} .kit-value`], ['type', '6', 'Enter'],
    ['menu', `${key('drive')} .kit-scrub`], ['click', item('Copy')],
    ['menu', `${key('output')} .kit-scrub`], ['click', item('Paste')],
    ['menu', `${key('drive')} .kit-scrub`], ['click', item('Return to')],
    ['menu', `${key('mix')} .kit-scrub`], ['click', item('Lock')]
  ]
  await onGallery('main', steps, async (tab) => {
    assert.is(await text(tab, `${key('output')} .kit-value`), '6.0 dB')
    assert.is(await text(tab, `${key('drive')} .kit-value`), '12.0 dB')
    assert.is(await tab.evaluate(
      `document.querySelector('${key('mix')}').dataset.locked`), 'true')
    assert.false(await has(tab, '.kit-pop'))

    // Locked, Mix sits out a preset that would move it.
    await tab.evaluate(`window.kit.session.apply(
      window.kit.store.get().values.map(() => 0)) ?? true`)
    assert.is(await text(tab, `${key('mix')} .kit-value`), '100 %')
    assert.is(await text(tab, `${key('tone')} .kit-value`), '200 Hz')
  })
})

test('a digit starts a value with no click first, and Delete is the default', async (assert) => {
  const steps = [['focus', `${key('drive')} .kit-scrub`], ['key', '2']]
  await onGallery('main', steps, async (tab) => {
    await tab.send('Input.insertText', { text: '4' })
    await STEPS.key(tab, ['Enter'])
    assert.is(await text(tab, `${key('drive')} .kit-value`), '24.0 dB')

    await STEPS.focus(tab, [`${key('drive')} .kit-scrub`])
    await STEPS.key(tab, ['Delete'])
    assert.is(await text(tab, `${key('drive')} .kit-value`), '12.0 dB')
    assert.is(types(await posted(tab)), 'begin set end begin set end')
  })
})

test('settings are chosen in a panel and kept for every editor', async (assert) => {
  const pick = (row, name) => ({
    sel: `[data-setting="${row}"] .kit-option`, text: name
  })
  const root = (name) => `document.documentElement.dataset.${name}`
  await onGallery('main', [['click', '.kit-settings']], async (tab) => {
    await click(tab, pick('density', 'Dense'))
    await click(tab, pick('theme', 'Light'))
    await click(tab, pick('peak-hold', 'Until pressed'))
    await click(tab, pick('motion', 'Still'))

    assert.is(await tab.evaluate(root('density')), 'dense')
    assert.is(await tab.evaluate(root('theme')), 'light')
    assert.is(await tab.evaluate(root('motion')), 'still')
    assert.deepEqual(await tab.evaluate('JSON.parse(localStorage.getItem("kit-prefs"))'),
      { hold: 'Until pressed', motion: 'Still', fine: 'With Shift', fall: 'Medium' })
    assert.deepEqual(await tab.evaluate(spills), [])

    await click(tab, pick('theme', 'Match'))
    assert.is(await tab.evaluate('localStorage.getItem("kit-theme")'), null)
    await STEPS.key(tab, ['Escape'])
    assert.false(await has(tab, '.kit-panel'))
    assert.deepEqual(await posted(tab), [])
  })
})

test('help only adds: the foot says the same, and a card says the rest', async (assert) => {
  const hover = ['hover', `${key('drive')} .kit-scrub`]
  const foot = 'How hard the signal is pushed into the shaper'
  const wait = (ms) => new Promise((resolve) => setTimeout(resolve, ms))
  await onGallery('main', [hover], async (tab) => {
    // Off: the foot explains, and no card comes.
    assert.is(await text(tab, '.kit-hint-about'), foot)
    await wait(700)
    assert.false(await has(tab, '.kit-tip'))

    await click(tab, '.kit-help')
    await STEPS.hover(tab, hover.slice(1))
    await wait(300)
    assert.is(await text(tab, '.kit-hint-about'), foot)
    assert.is(await text(tab, '.kit-tip b'), 'Drive')
    assert.is(await text(tab, '.kit-tip span'),
      `${foot}. From 0.0 dB to 36.0 dB. Rests at 12.0 dB.`)
    assert.is(await tab.evaluate(
      'document.querySelector(".kit-help").ariaPressed'), 'true')

    // And off again is as it was.
    await click(tab, '.kit-help')
    await STEPS.hover(tab, hover.slice(1))
    await wait(700)
    assert.is(await text(tab, '.kit-hint-about'), foot)
    assert.false(await has(tab, '.kit-tip'))
  })
})

test('the plugin\'s own bypass is one press, and says which it is', async (assert) => {
  await onGallery('main', [['click', '.kit-power']], async (tab) => {
    const sent = await posted(tab)

    assert.is(types(sent), 'begin set end')
    assert.deepEqual([sent[1].id, sent[1].value], [5, 1])
    assert.is(await tab.evaluate(
      'document.querySelector(".kit-power").ariaLabel'),
    'Bypassed. Press to turn on')
    assert.is(await tab.evaluate('document.documentElement.dataset.bypassed'), 'true')
  })
})

test('groups show one at a time, and every group\'s values in its tab', async (assert) => {
  const titles = `[...document.querySelectorAll('.kit-setting .kit-title')]
    .map((el) => el.textContent)`
  await onGallery('groups', [], async (tab) => {
    assert.deepEqual(await tab.evaluate(titles),
      ['Cutoff', 'Resonance', 'Low', 'High'])
    assert.is(await tab.evaluate('document.querySelectorAll(".kit-tab i").length'), 12)

    await tab.evaluate('document.querySelectorAll(".kit-tab")[0].click() ?? true')
    assert.deepEqual(await tab.evaluate(titles), ['Drive', 'Tone', 'Mix', 'Output'])
    assert.deepEqual(await tab.evaluate(spills), [])
  })
})

test('a fader is dragged up and down, as one gesture', async (assert) => {
  await onGallery('pairs', [], async (tab) => {
    await STEPS.drag(tab, [`${key('send')} .kit-fader-track`, 30])
    const sent = await posted(tab)

    assert.is(types(sent), 'begin set end')
    assert.is(sent[1].id, 14)
    assert.true(sent[1].value > PARAMS[14].default)
    assert.deepEqual(await tab.evaluate(spills), [])
  })
})

test('opposed settings trade: what one gains the other gives up', async (assert) => {
  await onGallery('pairs', [], async (tab) => {
    await STEPS.slide(tab, [`${key('mid')} .kit-oppose`, 40])
    const sent = await posted(tab)
    const [mid, side] = await tab.evaluate(
      '[6, 7].map((i) => window.kit.store.get().values[i])')

    assert.is(types(sent), 'begin begin set set end end')
    assert.true(mid > 0.5)
    assert.true(Math.abs(mid + side - 1) < 1e-9)
  })
})

test('linked settings keep their gap, until the link is let go', async (assert) => {
  const values = '[10, 11].map((i) => window.kit.store.get().values[i])'
  await onGallery('pairs', [], async (tab) => {
    await STEPS.slide(tab, [`${key('left')} .kit-scrub`, 30])
    const [left, right] = await tab.evaluate(values)
    assert.true(left > 0.12)
    assert.true(Math.abs(right - left - 0.02) < 1e-9)

    await click(tab, '.kit-link')
    await STEPS.slide(tab, [`${key('left')} .kit-scrub`, 20])
    const after = await tab.evaluate(values)
    assert.true(after[0] > left)
    assert.is(after[1], right)
  })
})

test('a span moves by either end, or whole by its middle', async (assert) => {
  const values = '[8, 9].map((i) => window.kit.store.get().values[i])'
  await onGallery('pairs', [], async (tab) => {
    const [low, high] = await tab.evaluate(values)
    await STEPS.slide(tab, [`${key('low')} .kit-span-fill`, 30])
    const moved = await tab.evaluate(values)
    assert.true(moved[0] > low)
    assert.true(Math.abs((moved[1] - moved[0]) - (high - low)) < 1e-9)

    await STEPS.slide(tab, [`${key('low')} .kit-span-hi`, 20])
    const stretched = await tab.evaluate(values)
    assert.is(stretched[0], moved[0])
    assert.true(stretched[1] > moved[1])
  })
})

test('the pad moves two settings with one point', async (assert) => {
  await onGallery('pairs', [], async (tab) => {
    await click(tab, '.kit-pad-dot')
    const before = (await posted(tab)).length
    await STEPS.slide(tab, ['.kit-pad-dot', 30])
    const sent = (await posted(tab)).slice(before)

    assert.is(types(sent.slice(0, 2)), 'begin begin')
    assert.is(types(sent.slice(-2)), 'end end')
    assert.true(await tab.evaluate('window.kit.store.get().values[12]') > 0.6)
  })
})

test('a choice of many opens a list in the house style', async (assert) => {
  const chosen = `${key('mode')} .kit-chosen`
  await onGallery('lists', [['click', '.kit-chooser']], async (tab) => {
    assert.is(await tab.evaluate(
      'document.querySelectorAll(".kit-pop button").length'), 6)
    await click(tab, { sel: '.kit-pop button', text: 'Lydian' })
    const sent = await posted(tab)

    assert.is(types(sent), 'begin set end')
    assert.deepEqual([sent[1].id, sent[1].value], [15, 0.6])
    assert.is(await text(tab, chosen), 'Lydian')
    assert.false(await has(tab, '.kit-pop'))

    // The arrows step through it without opening it.
    await STEPS.focus(tab, ['.kit-chooser'])
    await STEPS.key(tab, ['ArrowDown'])
    assert.is(await text(tab, chosen), 'Mixolydian')
    assert.deepEqual(await tab.evaluate(spills), [])
  })
})

test('a switch says what it is and which way it is', async (assert) => {
  await onGallery('lists', [], async (tab) => {
    assert.is(await text(tab, '.kit-switch-said'), 'Freeze off')
    await click(tab, '.kit-switch')
    const sent = await posted(tab)

    assert.deepEqual([sent[1].id, sent[1].value], [16, 1])
    assert.is(await text(tab, '.kit-switch-said'), 'Freeze on')
    assert.is(await tab.evaluate(
      'document.querySelector(".kit-switch").ariaChecked'), 'true')
  })
})

test('a preset from a newer version asks before it loads what fits', async (assert) => {
  const steps = [
    ['click', '.kit-preset'],
    ['click', { sel: '.kit-preset-pick', text: 'From the future' }]
  ]
  const pick = (name) => ({ sel: '.kit-notice .kit-option', text: name })
  await onGallery('main', steps, async (tab) => {
    assert.contains(await text(tab, '.kit-notice-title'), 'From the future')
    assert.contains(await text(tab, '.kit-notice p'),
      '2 of its settings are not known here: sparkle, air.')
    // Nothing has moved, and the name is still the old one.
    assert.is(await text(tab, `${key('drive')} .kit-value`), '12.0 dB')
    assert.is(await text(tab, '.kit-preset-at'), 'Init')

    await click(tab, pick('Load what fits'))
    assert.false(await has(tab, '.kit-notice'))
    assert.is(await text(tab, `${key('drive')} .kit-value`), '20.0 dB')
    assert.is(await text(tab, '.kit-preset-at'), 'From the future')
  })
})

test('a notice cancelled leaves everything as it was', async (assert) => {
  const steps = [
    ['click', '.kit-preset'],
    ['click', { sel: '.kit-preset-pick', text: 'From the future' }],
    ['click', { sel: '.kit-notice .kit-option', text: 'Cancel' }]
  ]
  await onGallery('main', steps, async (tab) => {
    assert.false(await has(tab, '.kit-notice'))
    assert.is(await text(tab, `${key('drive')} .kit-value`), '12.0 dB')
    assert.true(await tab.evaluate('document.querySelector(".kit-undo").disabled'))
    assert.is(await tab.evaluate(
      'getComputedStyle(document.querySelector(".kit-picture")).display'),
    'block')
  })
})

test('where the host keeps presets as files, the page asks it to', async (assert) => {
  const files = {
    type: 'presets',
    list: [{ name: 'From disk', values: { drive: 0.75 } }],
    dir: 'C:/Presets'
  }
  const steps = [['tell', files], ['click', '.kit-preset'], ['click', '.kit-save']]
  const names = `[...document.querySelectorAll('.kit-preset-name')]
    .map((el) => el.textContent)`
  await onGallery('main', steps, async (tab) => {
    assert.true((await tab.evaluate(names)).includes('From disk'))
    await tab.send('Input.insertText', { text: 'Kept' })
    await STEPS.key(tab, ['Enter'])
    const saved = (await posted(tab)).at(-1)

    assert.is(saved.type, 'preset-save')
    assert.is(saved.name, 'Kept')
    assert.is(saved.values.drive, PARAMS[0].default)
    // Shown at once, and nothing is kept by the page itself.
    assert.true((await tab.evaluate(names)).includes('Kept'))
    assert.is(await tab.evaluate('localStorage.getItem("kit-presets:Gallery")'), null)

    await click(tab, '.kit-preset-drop')
    const dropped = (await posted(tab)).at(-1)
    assert.is(dropped.type, 'preset-delete')
  })
})

test('a setting says what it has learned, is waiting for, or is locked against', (assert) => {
  const learn = { waiting: 4, map: { 9: 74, 4: 1 } }
  const cases = [
    [9, learn, false, ' (CC 74)'],
    [9, learn, true, ' (CC 74, locked)'],
    [4, learn, false, ' (move a controller, CC 1)'],
    [2, learn, false, ''],
    [2, null, true, ' (locked)'],
    [2, null, false, '']
  ]

  cases.forEach(([id, told, locked, want]) =>
    assert.is(tagFor(id, told, locked), want))
  assert.is(EMPTY.learn, null)
  assert.deepEqual(reduce(EMPTY, { type: 'learn', waiting: null }).learn,
    { waiting: null, map: {} })
})

test('a setting learns a controller from its menu, and says so', async (assert) => {
  const item = (name) => ({ sel: '.kit-pop button', text: name })
  const tag = `${key('drive')} .kit-tag`
  const told = (waiting, map) => ['tell', { type: 'learn', waiting, map }]
  const items = `[...document.querySelectorAll('.kit-pop button')]
    .map((el) => el.querySelector('span').textContent)`
  await onGallery('main', [['menu', `${key('drive')} .kit-scrub`]], async (tab) => {
    // A host that has said nothing of learning is not offered it.
    assert.false((await tab.evaluate(items)).some((name) => /controller/.test(name)))
    await STEPS.key(tab, ['Escape'])

    await STEPS.tell(tab, told(null, {}).slice(1))
    await STEPS.menu(tab, [`${key('drive')} .kit-scrub`])
    await click(tab, item('Learn a controller'))
    assert.deepEqual((await posted(tab)).at(-1), { type: 'learn', id: 0 })

    // The host says it is waiting, then that controller 74 was moved.
    await STEPS.tell(tab, told(0, {}).slice(1))
    assert.is(await text(tab, tag), ' (move a controller)')
    assert.is(await tab.evaluate(
      `document.querySelector('${key('drive')}').dataset.learning`), 'true')
    await STEPS.key(tab, ['Escape'])
    assert.deepEqual((await posted(tab)).at(-1), { type: 'learn', id: null })

    await STEPS.tell(tab, told(null, { 0: 74 }).slice(1))
    assert.is(await text(tab, tag), ' (CC 74)')
    await STEPS.menu(tab, [`${key('drive')} .kit-scrub`])
    await click(tab, item('Forget controller 74'))
    assert.deepEqual((await posted(tab)).at(-1), { type: 'forget', id: 0 })
  })
})

test('level matching is one press in the header, and says which way it is', async (assert) => {
  const label = 'document.querySelector(".kit-match").ariaLabel'
  await onGallery('main', [], async (tab) => {
    assert.is(await tab.evaluate(label),
      'Match the level going out to the level coming in')
    await click(tab, '.kit-match')
    const sent = await posted(tab)

    assert.is(types(sent), 'begin set end')
    assert.deepEqual([sent[1].id, sent[1].value], [17, 1])
    assert.is(await tab.evaluate(label),
      'Level matched to what comes in. Press to stop')
    assert.is(await tab.evaluate(
      'document.querySelector(".kit-match").ariaPressed'), 'true')
    // It is the header's, not one more switch among the settings.
    assert.is(await tab.evaluate(
      'document.querySelectorAll(".kit-sheet [data-key=match]").length'), 0)
    assert.deepEqual(await tab.evaluate(spills), [])
  })
})

test('a long note under the name gives way: the tools stay on the glass', async (assert) => {
  const fits = `(() => {
    const glass = document.querySelector('.kit-glass').getBoundingClientRect()
    const last = document.querySelector('.kit-power').getBoundingClientRect()
    return glass.right - last.right
  })()`
  await onGallery('main', [], async (tab) => {
    await tab.evaluate(`(document.querySelector('.kit-note').textContent =
      'A note a good deal longer than the header has any room for') && true`)
    assert.true(await tab.evaluate(fits) >= 20)
    assert.deepEqual(await tab.evaluate(spills), [])
  })
})

test('a bar never falls under the level, and a wide text is scaled to fit', (assert) => {
  assert.is(fallen(0.9, 0.3, 0.5, 20 / 60), 0.9 - 10 / 60)
  assert.is(fallen(0.9, 0.3, 60, 20 / 60), 0.3)
  assert.is(fallen(0.2, 0.3, 1, 20 / 60), 0.3)
  assert.is(fallen(0.9, 0.3, -1, 20 / 60), 0.9)

  assert.is(textFit(80, 100), 1)
  assert.is(textFit(200, 100), 0.6)
  assert.is(textFit(125, 100), 0.8)
  assert.is(textFit(0, 100), 1)
})

// Where everything on the page is, to tell whether anything moved.
const places = `JSON.stringify([...document.querySelectorAll(
  '.kit-glass, .kit-sheet, .kit-foot, .kit-control, .kit-tool, .kit-picture'
)].map((el) => {
  const r = el.getBoundingClientRect()
  return [r.left, r.top, r.width, r.height].map((v) => Math.round(v * 10))
}))`

test('pointing at anything moves nothing, at any density, help on or off', async (assert) => {
  const over = [
    `${key('drive')} .kit-scrub`, `${key('tone')} .kit-scrub`,
    `${key('output')} .kit-scrub`, '.kit-option', '.kit-undo', '.kit-settings',
    '.kit-match', '.kit-power', '.kit-preset', `${key('input')}`
  ]
  await onGallery('main', [], async (tab) => {
    for (const help of [false, true]) {
      if (help) await click(tab, '.kit-help')
      for (const density of ['roomy', 'regular', 'dense']) {
        await STEPS.away(tab, [])
        const before = await tab.evaluate(places)
        for (const spec of over) {
          await STEPS.hover(tab, [spec])
          await new Promise((resolve) => setTimeout(resolve, 160))
          assert.is(await tab.evaluate(places), before,
            `${spec}, ${density}, help ${help}`)
        }
        await click(tab, '.kit-density')
      }
    }
    assert.deepEqual(await tab.evaluate(spills), [])
  })
})

test('a hint too long for its two lines is drawn smaller, in the same box', async (assert) => {
  const size = 'parseFloat(document.querySelector(".kit-hint").style.fontSize)'
  const height = 'document.querySelector(".kit-hint").getBoundingClientRect().height'
  await onGallery('main', [['hover', `${key('drive')} .kit-scrub`]], async (tab) => {
    const tall = await tab.evaluate(height)
    assert.is(await tab.evaluate(size), 13)

    await STEPS.hover(tab, [`${key('tone')} .kit-scrub`])
    assert.true(await tab.evaluate(size) < 13)
    assert.true(await tab.evaluate(size) >= 10.5)
    assert.is(await tab.evaluate(height), tall)
  })
})

test('every tooltip is the house\'s own, and a press puts it away', async (assert) => {
  const wait = (ms) => new Promise((resolve) => setTimeout(resolve, ms))
  await onGallery('main', [], async (tab) => {
    // Nothing anywhere would make the browser draw one of its own.
    assert.is(await tab.evaluate('document.querySelectorAll("[title]").length'), 0)

    await STEPS.hover(tab, ['.kit-undo'])
    assert.false(await has(tab, '.kit-tip'))
    await wait(600)
    assert.is(await text(tab, '.kit-tip b'), 'Undo (Ctrl+Z)')
    assert.false(await has(tab, '.kit-tip span'))

    await click(tab, '.kit-help')
    assert.false(await has(tab, '.kit-tip'))
    await STEPS.hover(tab, ['.kit-undo'])
    await wait(300)
    assert.contains(await text(tab, '.kit-tip span'), 'Steps back')
    // It stays on the page.
    assert.deepEqual(await tab.evaluate(spills), [])
  })
})

test('a meter holds its number and lets its bar fall at a pace', async (assert) => {
  const reading = (level, said) => [{
    type: 'meters', levels: [level, 0.5], levelTexts: [said, '-30 dB']
  }]
  const fill = (n) => `Number(getComputedStyle(document.querySelectorAll(
    '[data-key="level"] .kit-segment')[${n}]).getPropertyValue('--fill'))`
  const wait = (ms) => new Promise((resolve) => setTimeout(resolve, ms))
  await onGallery('main', [], async (tab) => {
    await STEPS.tell(tab, reading(0.9, '-6 dB'))
    await STEPS.tell(tab, reading(0.3, '-42 dB'))
    await wait(100)

    // The reading dropped 36 dB at once; the bar has only begun to.
    assert.is(await text(tab, '[data-key="level"] .kit-value'), '-6 dB')
    assert.true(await tab.evaluate(fill(1)) > 0.5)

    await wait(2400)
    assert.is(await tab.evaluate(fill(1)), 0)
    assert.true(Math.abs(await tab.evaluate(fill(0)) - 0.5) < 0.01)
    // The hold has run out too, and the number says where it is now.
    assert.is(await text(tab, '[data-key="level"] .kit-value'), '-42 dB')
  })
})

test('a number too wide for its column is drawn smaller, not over its neighbour', async (assert) => {
  const texts = PARAMS.map((row, i) => (i === 1 ? '16000.0 Hz' : 'x'))
  const state = { type: 'state', values: PARAMS.map((r) => r.default), texts }
  const fit = `Number(document.querySelector('${key('tone')} .kit-value')
    .style.getPropertyValue('--fit'))`
  const inside = `(() => {
    const box = document.querySelector('${key('tone')}').getBoundingClientRect()
    const unit = document.querySelector('${key('tone')} .kit-unit')
      .getBoundingClientRect()
    return box.right - unit.right
  })()`
  await onGallery('main', [['tell', state]], async (tab) => {
    await tab.evaluate('new Promise((r) => requestAnimationFrame(() => r(true)))')
    assert.true(await tab.evaluate(fit) < 1)
    assert.true(await tab.evaluate(inside) >= -1)
  })
})

test('the windows draw the curves the sound is made with', (assert) => {
  // The same numbers are in kit/dsp/src/tube.rs and transformer.rs.
  const tube = [
    [0.5, 0, 0.462117], [0.5, 1, 0.423433], [2, 0.5, 0.891878],
    [-2, 1, -0.964028], [3, 1, 0.666502]
  ]
  const staged = [
    [0.5, 0, 0, 0.480953], [-1, 0, 0, -0.712415], [-3, 0, 0, -0.931110],
    [1, 1, 0, 0.618432], [0.5, 0, 1, 0.470160], [-2, 0.5, 1, -0.949105]
  ]
  const iron = [
    [0.5, 4, 0, 0.246240], [0.5, 4, 1, 0.241007], [0.2, 2, 0.5, 0.194357],
    [-1, 4, 1, -0.249832], [0.3, 0, 1, 0.3]
  ]
  const close = (got, want) => Math.abs(got - want) < 1e-5

  tube.forEach(([x, grid, want]) =>
    assert.true(close(tubeCurve(x, grid), want), `tube ${x} ${grid}`))
  staged.forEach(([x, grid, stage, want]) => assert.true(
    close(tubeCurve(x, grid, stage), want), `tube ${x} ${grid} ${stage}`))
  iron.forEach(([x, push, steel, want]) => assert.true(
    close(ironCurve(x, push, steel), want), `iron ${x} ${push} ${steel}`))
  // At rest a valve is tanh and a transformer is a wire.
  assert.is(tubeCurve(0.7, 0), Math.tanh(0.7))
  assert.is(ironCurve(0.7, 0), 0.7)
})

test('a valve and a transformer each have a window, the same everywhere', async (assert) => {
  const titles = `[...document.querySelectorAll('.kit-panel .kit-setting .kit-title')]
    .map((el) => el.firstChild.textContent)`
  const moved = (kind) =>
    `document.querySelector('.kit-open-${kind}').dataset.moved`
  const inked = `(() => {
    const c = document.querySelector('.kit-tone-curve canvas')
    const d = c.getContext('2d').getImageData(0, 0, c.width, c.height).data
    let sum = 0
    for (let i = 3; i < d.length; i += 4) sum += d[i]
    return sum
  })()`
  const frame = 'new Promise((r) => requestAnimationFrame(() => r(true)))'
  await onGallery('tone', [['click', '.kit-open-tube']], async (tab) => {
    assert.is(await text(tab, '.kit-panel-title'), 'Tube')
    assert.deepEqual(await tab.evaluate(titles),
      ['Bias', 'Grid', 'Bloom', 'Feedback', 'Sag', 'Glow'])
    // How it is wired is a choice in the window's bar, not a number.
    assert.is(await text(tab,
      `.kit-panel-bar ${key('tube_stage')} [aria-pressed="true"]`), 'Pentode')
    assert.is(await tab.evaluate(moved('tube')), 'false')
    await tab.evaluate(frame)
    const before = await tab.evaluate(inked)
    assert.true(before > 0)

    // A setting in the window is a setting like any other, the curve
    // follows it, and the way in says something is no longer at rest.
    await STEPS.dblclick(tab, [`${key('tube_grid')} .kit-value`])
    await STEPS.type(tab, ['100', 'Enter'])
    await tab.evaluate(frame)
    await tab.evaluate(frame)
    const sent = await posted(tab)
    assert.deepEqual([sent[1].id, sent[1].value], [21, 1])
    assert.not(await tab.evaluate(inked), before)
    assert.is(await tab.evaluate(moved('tube')), 'true')
    assert.deepEqual(await tab.evaluate(spills), [])

    // "At rest" puts the whole window back as one step to undo.
    await click(tab, '.kit-rest')
    assert.is(await text(tab, `${key('tube_grid')} .kit-value`), '0 %')
    assert.is(await tab.evaluate(moved('tube')), 'false')
    await click(tab, '.kit-done')

    await click(tab, '.kit-open-transformer')
    assert.is(await text(tab, '.kit-panel-title'), 'Transformer')
    assert.deepEqual(await tab.evaluate(titles),
      ['Drive', 'Size', 'Iron', 'Source', 'Grip', 'Top', 'Ring'])
    assert.deepEqual(await tab.evaluate(spills), [])
  })
})

test('a plugin with neither has no way into a window, and theirs stay off the sheet', async (assert) => {
  await onGallery('main', [], async (tab) => {
    // The gallery has both, so both ways in are there, but none of
    // their settings is among the sheet's own.
    assert.true(await has(tab, '.kit-open-tube'))
    assert.is(await tab.evaluate(
      'document.querySelectorAll(".kit-values [data-key^=tube_]").length'), 0)
  })
})

test('either half is soloed from the glass, and one control weighs them', async (assert) => {
  const pressed = `document.querySelector(
    '.kit-listen .kit-option[aria-pressed="true"]').textContent`
  await onGallery('tone', [], async (tab) => {
    assert.is(await tab.evaluate(pressed), 'Both')
    await click(tab, { sel: '.kit-listen .kit-option', text: 'Side' })
    const sent = await posted(tab)

    assert.deepEqual([sent[1].id, sent[1].value], [19, 1])
    assert.is(await tab.evaluate(pressed), 'Side')
    // The balance rests in the middle and fills from there.
    assert.true(await has(tab, `${key('balance')}.kit-bipolar`))
    assert.deepEqual(await tab.evaluate(spills), [])
  })
})

test('a switch among a foot\'s pills is one pill, pressed while on', async (assert) => {
  const pressed = `document.querySelector('${key('key')} .kit-option')
    .getAttribute('aria-pressed')`
  await onGallery('tone', [], async (tab) => {
    assert.is(await tab.evaluate(pressed), 'false')
    await click(tab, `${key('key')} .kit-option`)
    const sent = await posted(tab)

    assert.deepEqual([sent[1].id, sent[1].value], [28, 1])
    assert.is(await tab.evaluate(pressed), 'true')
    assert.is(await text(tab, `${key('key')} .kit-option`), 'Sidechain')
    assert.deepEqual(await tab.evaluate(spills), [])
  })
})

test('the foot\'s first words fit their two lines before anything is pointed at', async (assert) => {
  // The tone page's foot is the fullest: a switch and two windows.
  await onGallery('tone', [], async (tab) => {
    await tab.evaluate(
      'new Promise((r) => requestAnimationFrame(() => r(true)))')
    const [full, box] = await tab.evaluate(`(() => {
      const el = document.querySelector('.kit-hint')
      return [el.scrollHeight, el.clientHeight]
    })()`)
    assert.true(full <= box + 1, `${full} in ${box}`)
  })
})

test('a character sets the whole window in one step, and is named while it holds', async (assert) => {
  const said = (tab) => text(tab, '.kit-character')
  const pick = async (tab, name) => {
    await click(tab, '.kit-character')
    await click(tab, { sel: '.kit-pop button', text: name })
  }
  await onGallery('tone', [['click', '.kit-open-tube']], async (tab) => {
    assert.is(await said(tab), 'Character')
    await pick(tab, 'Starved plate')

    assert.is(await said(tab), 'Starved plate')
    assert.is(await text(tab, `${key('tube_bias')} .kit-value`), '-60 %')
    assert.is(await text(tab,
      `${key('tube_stage')} [aria-pressed="true"]`), 'Triode')
    // What a character does not name goes to its rest.
    assert.is(await text(tab, `${key('tube_glow')} .kit-value`), '0 %')
    // Nothing outside the window moves.
    await click(tab, '.kit-done')
    assert.is(await text(tab, `${key('drive')} .kit-value`), '12.0 dB')
    await click(tab, '.kit-open-tube')

    // Refined from there it is no longer that character.
    await STEPS.dblclick(tab, [`${key('tube_sag')} .kit-value`])
    await STEPS.type(tab, ['10', 'Enter'])
    assert.is(await said(tab), 'Character')
    // And one undo is the whole character.
    await click(tab, '.kit-undo')
    assert.is(await said(tab), 'Starved plate')
    await click(tab, '.kit-undo')
    assert.is(await text(tab, `${key('tube_bias')} .kit-value`), '0 %')
    assert.deepEqual(await tab.evaluate(spills), [])
  })
})

test('the transformer window names the kinds of iron it stands for', async (assert) => {
  await onGallery('tone', [['click', '.kit-open-transformer']], async (tab) => {
    await click(tab, '.kit-character')
    const names = await tab.evaluate(
      '[...document.querySelectorAll(".kit-pop button")].map((b) => b.textContent)')
    assert.deepEqual(names, [
      'Mic output, T14 kind', 'Ouncer, O-12 kind',
      'Gapped output, LO1166 kind', 'Clean nickel input', 'Big steel output'
    ])
    await click(tab, { sel: '.kit-pop button', text: 'Gapped output, LO1166 kind' })
    assert.is(await text(tab, `${key('iron_grip')} .kit-value`), '60 %')
    assert.is(await text(tab, `${key('iron_size')} .kit-value`), '30 Hz')
    assert.deepEqual(await tab.evaluate(spills), [])
  })
})
