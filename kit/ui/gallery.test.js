// The rest of the kit's page, driven on the gallery: presets, a
// value's menu, the panels, help, bypass, groups, the fader, paired
// controls and the pad. Each is held to what docs/benchmarks.md says
// the best of its kind does.

import test from 'mvt'
import {
  NO_HISTORY, TIES, fingerprint, helpFor, holdFor, prefsFor, presetMoves,
  presetValues, recorded, redone, undone
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
    { hold: '2 s', motion: 'Full', fine: 'With Shift' })
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
      { hold: 'Until pressed', motion: 'Still', fine: 'With Shift' })
    assert.deepEqual(await tab.evaluate(spills), [])

    await click(tab, pick('theme', 'Match'))
    assert.is(await tab.evaluate('localStorage.getItem("kit-theme")'), null)
    await STEPS.key(tab, ['Escape'])
    assert.false(await has(tab, '.kit-panel'))
    assert.deepEqual(await posted(tab), [])
  })
})

test('with help on, the foot says a setting\'s range and resting place', async (assert) => {
  const steps = [['click', '.kit-help'], ['hover', `${key('drive')} .kit-scrub`]]
  await onGallery('main', steps, async (tab) => {
    assert.is(await text(tab, '.kit-hint-about'),
      'How hard the signal is pushed into the shaper.' +
      ' From 0.0 dB to 36.0 dB. Rests at 12.0 dB.')
    assert.is(await tab.evaluate(
      'document.querySelector(".kit-help").ariaPressed'), 'true')
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
