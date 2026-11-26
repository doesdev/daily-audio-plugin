# Held to the best of the field

The cornerstone rule of this repo:

> Every interaction and every feature is compared with what the
> leaders of the field ship, and must meet or exceed the experience
> and the quality of that benchmark. Where it does not, it says so,
> says why, and says what is next.

It does not mean copying. It means that before something is built we
know who does it best and what makes theirs good to use, and after it
is built we can show, with a test, that ours is at least that good.

## What a benchmark is

One thing a person does: set a value, find a preset, read a level,
undo a mistake. For each:

- **Leaders.** One or more products that do it best, and what exactly
  they do. A claim taken from the maker's own manual or page carries
  its `source`. A claim from memory has `"source": null`; the check
  counts those, and whoever next touches the control looks them up.
- **Ours.** What the kit or the plugin does, in the same terms.
- **Verdict.** `exceeds`, `meets` or `under`.
- **Proof.** For `meets` and `exceeds`, the names of the tests that
  show it. No test, no claim.
- **Gap and next.** For `under`, what is missing and the change that
  would close it.

## Where they live

- `kit/benchmarks.json` holds one for everything the kit's page does.
- `plugins/NAME/benchmark.json` compares a plugin with at least two of
  the best of its kind, on the seven points in `docs/design.md` ("The
  bar"): picture, precision, compare, explains, fits, character,
  presets. A verdict and a note on each.

## How it is enforced

`npm run check` runs `scripts/benchmarks.js`, and fails when:

- something the kit's page builds has no benchmark covering it;
- a benchmark claims `meets` or `exceeds` with no test, or names a
  test that does not exist;
- a benchmark is `under` without its gap and what is next;
- a benchmark names no leader, or a leader without what they do;
- a plugin does not ship a Mix and level matching and does not say,
  under `tools`, why one makes no sense for it;
- a plugin does not say, under `tools`, whether it has mid and side,
  a valve or a transformer, or says it has one without the kit's
  tools for it;
- a plugin has no `benchmark.json`, has one copied from another
  plugin, names fewer than two leaders, or is silent on any of the
  seven points.

It prints the tally and every gap on each run, so what is under the
bar is read every day and not filed away.

## In practice

**Before building** a control or a feature: write its entry. Name the
leaders and what they do; read their manual for it, not a memory of
it. Decide what meeting it means here, and write the tests' names.

**While building**: the tests are the benchmark made checkable. "Type
a value with no click first", "a preset is one step to undo", "a clip
stays lit until clicked" are each a line in a leader's manual turned
into a test of ours.

**Before committing**: look at it in the gallery (`npm run gallery`)
in both themes, beside a picture of the leader's. Set the verdict
honestly. `under` with a plain gap is a pass; `meets` without a test
is a failure.

**Each daily run**: read the tally. A new plugin's `benchmark.json`
is written before its editor, and its `under` points go in its
`NOVELTY.md`. If the plugin needs something the kit is `under` on,
closing that gap in the kit comes first, in its own commit.

## What exceeding looks like

Meeting is the floor. The entries to be proudest of are those where
ours does something the leaders do not: the range and the default
shown while a value is held, a preset told apart by its four bars and
heard by pointing at it, a tab that still shows the values behind it.
When an entry earns `exceeds`, its `ours` says what the extra is.
