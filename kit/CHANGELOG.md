# The kit: what changed

What changed in the kit every plugin is built on, for whoever writes
one. An entry a day, newest first. What a person using a plugin sees
is in that plugin's own `CHANGELOG.md`.

## 2026-11-28

- Fixed: a linked setting moved in the moment after it was linked
  could be told to no one and the two left apart. A peer is sent
  where the setting is as soon as it shows it has joined.
- Fixed, on Windows: saying "here" to ports no one holds left a
  refusal for each in front of what peers had said, and reading
  stopped at the first. They are stepped over. This is what made the
  linking tests fail now and then in CI.
- The proof that a costly tail is caught no longer leans on what a
  denormal costs the machine it runs on, which on CI's is very
  little; a second proof shows a plugin never meets one at all.
- Fixed: dropouts at small buffers. In VST3 every meter that moved
  was told to the host every block, thousands of times a second at a
  buffer of 32; meters are now told thirty times a second at most.
- The suite budgets every plugin at buffers of 16 and 32, and fails
  one whose reset takes more than a quarter of the smallest.
- A link that is asked for is asked again until it takes, so a lost
  message no longer leaves a box unticked.
- Fixed: an instance that has gone quiet no longer costs more than
  one that is playing. The runner flushes denormals for each block,
  and the suite fails a plugin whose tail costs more than its sound.
  `docs/performance.md` has what was measured and what comes next.
- Linking lists each instance by the track it is on, as the host
  names it, in VST3 and CLAP.

## 2026-11-27

- Changelogs: one for the kit and one for each plugin, checked, and
  quoted in every release's notes.
- Linking: any setting moves with the same setting in other DAP
  instances, across different plugins, from a right-click. Bypass
  links from the power button, a tube or transformer window whole.
  Links are saved with the project.
- The name is Daily Audio Plugin, DAP for short, and it is every
  plugin's vendor. The mark heads every editor, with the plugin's name
  centred under it in purple.
- `const CATEGORY` says what kind of thing a plugin is, so a host
  files it there and not under "Other".
- Fixed: a VST3 host that fails a short read at the end of a saved
  state no longer loses the state, which reset every setting when a
  project was reopened.
- Fixed: a right-click with no menu of ours opens no browser menu.

## 2026-11-26

- The tube and transformer windows, the same in every plugin that
  models one, each with a lever for a cause that makes parts differ,
  and characters as named settings of them.
- A sidechain input in both formats, and one shared Sidechain switch.
- Mid and side tools: Both, Mid and Side to hear either alone, and a
  Balance.
- A Mix and level matching for every plugin they make sense for.
- Nothing moves when it is pointed at, hints are the kit's own and
  not the browser's, a meter falls at a chosen rate, and the page
  fills its window.
- Fixed: a setting with a wide range moves across all of it when
  dragged, not a step at a time.
- Fixed: release zips open in Windows' own zip handling.

## 2026-11-25

- The kit: VST3 and CLAP from one plugin, an editor in a web view,
  and a suite every plugin runs.
- The look: a dark glass over a sheet, numbers as the controls,
  purple for what is touched, light and dark.
- Bypass the host knows, presets as files, undo, A and B, MIDI learn,
  and the sound itself drawn in the editor.
- Every feature held to the best of its field, in a register the
  check reads.
