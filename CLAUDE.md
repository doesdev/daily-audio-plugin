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
```

## Rules

- Tests first. A plugin's `tests/sound.rs` fails before its sound is
  written.
- Never weaken a check in `kit/test` to pass a plugin.
- Never change an id that has shipped.
- `process` never allocates, locks or waits.
- Kit changes go in their own commit with their own tests.
- Look at every screenshot before committing it, in both themes.
- An editor follows `docs/design.md`: monotone, purple only for a
  value, other colours only where they mean something, and none of
  the habits it lists under "What it avoids".
- Say what was not verified. Off Windows that is the WebView2 editor
  tests and both validators.
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
