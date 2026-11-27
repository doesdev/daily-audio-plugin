# daily-audio-plugin

A new audio plugin every day on one shared kit. Read `README.md` for
the layout, `docs/kit.md` for how a plugin is written, `docs/design.md`
for how an editor looks, and
`docs/daily-run.md` before making one: it is the procedure, and it is
not optional.

## Commands

```sh
npm ci
npm run check                      # everything; CI runs exactly this
cargo test -p NAME                 # one plugin
npm run new -- NAME "Display Name"
npm run shots -- NAME
npm run docs -- NAME
npm run gallery                    # the kit's own pictures, both themes
```

## Rules

- Tests first. A plugin's `tests/sound.rs` fails before its sound is
  written.
- Never weaken a check in `kit/test` to pass a plugin.
- Never change an id that has shipped.
- `process` never allocates, locks or waits.
- Kit changes go in their own commit with their own tests.
- Look at every screenshot before committing it, in both themes.
- Every interaction and feature is compared with what the leaders of
  the field ship and meets or exceeds it, or says plainly where it is
  under and what is next (`docs/benchmarks.md`). Nothing goes in the
  kit without an entry in `kit/benchmarks.json`, no "meets" without
  a test, and every plugin has a `benchmark.json`. The check fails
  otherwise.
- Every plugin ships a Mix and level matching wherever they make
  sense, and says in its `benchmark.json` why not where they do not.
- A plugin that works on mid and side ships the kit's solo and
  balance; one that models a valve or a transformer uses the kit's,
  with its shared window (`docs/tone.md` is why its levers are what
  they are). None of these is written twice.
- An editor is built round a picture only that plugin has, and says
  in `NOVELTY.md` where it is under the bar.
- An editor follows `docs/design.md`: mostly one tone with depth,
  purple only for what is being touched, any other colour only when it
  is news, and none of the habits listed under "What it avoids".
- Say what was not verified. Off Windows that is the WebView2 editor
  tests and both validators.
- Every change is written down: a plugin's in its `CHANGELOG.md`
  under the version that ships it, the kit's in `kit/CHANGELOG.md`
  under the day. Say what a person will notice, not what was edited.
  A release quotes them. The check fails code that changed without a
  line.
- The brand is Daily Audio Plugin, DAP for short (`docs/brand.md`).
  Every plugin's vendor is `kit::VENDOR`.
- No em or en dashes in any doc.

## Style

- Rust: `cargo fmt`, clippy clean with `-D warnings`. Comments say why,
  in plain words. Test names are sentences about behaviour. Table
  driven tests where cases share a shape.
- JavaScript: StandardJS, no semicolons, lines to 80 characters,
  functional, arrow arguments always in parentheses, tests with `mvt`.
  No framework and no build step in `kit/ui`.
- Scripts are kebab-case with *nix-style flags.

## What runs where

The daily run happens on Linux. There the Rust side, the page (in a
headless browser), the screenshots and the manuals are all checked.
The editor in a real WebView2, pluginval and clap-validator only run
in CI, on Windows. A green check here is not the whole story until CI
agrees.
