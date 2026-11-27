# Daily Audio Plugin

DAP: a new audio plugin every day, each doing something no plugin found
does yet. VST3 and CLAP, Windows 11 x64. Every plugin is in this one
repo, built on one kit, and released from here.

## Layout

- `kit/`: what every plugin is made of.
  - `core/` (`kit`): a plugin's spec, its parameters, its saved state
    and the one `process` call.
  - `dsp/` (`kit-dsp`): tested building blocks for the audio thread.
  - `vst3/`, `clap/`: any kit plugin as a VST3 or a CLAP, on raw
    bindings.
  - `webui/`: the editor in a system webview, and what the page and
    the plugin say to each other.
  - `ui/`: the page every editor starts from, and its controls. Plain
    ES modules, no framework and no build step.
  - `test/` (`kit-test`): the checks every plugin is held to.
- `plugins/`: one folder a plugin. `template/` is the one new plugins
  are copied from; it is not released.
- `scripts/`: making a plugin, its screenshots and manual, the check,
  bundling and releasing.
- `ideas/`: what might be made, and a ledger of what was.
- `docs/`: how a plugin is written (`kit.md`), how an editor looks
  (`design.md`), how the daily run goes (`daily-run.md`), and why things are the way they are
  (`decisions/`).

## A plugin

```sh
npm run new -- tape-bloom "Tape Bloom"
```

makes `plugins/tape-bloom` from the template under its own names and
its own ids. A plugin is:

| File | What it is |
|------|------------|
| `NOVELTY.md` | What is new about it, and what was found looking for it elsewhere |
| `src/lib.rs` | The spec (settings, meters, size, ids) and the sound |
| `tests/suite.rs` | One line: the checks every plugin must pass |
| `tests/sound.rs` | What this plugin itself promises, written first |
| `ui/main.js`, `ui/style.css` | The editor, composed from the kit's controls |
| `docs/scenes.json` | The states of the editor the screenshots show |
| `docs/manual.md`, `docs/images/` | The manual and its pictures |
| `examples/demo.rs` | WAV demos to listen to |

`docs/kit.md` says how each is written.

## Develop

```sh
npm ci
npm run check
```

Runs `cargo fmt --check`, `cargo clippy`, `cargo test`, on Windows the
editor in a real WebView2, the scripts' and the page's tests (the page
in a headless browser), a check that every dependency's licence allows
shipping, each manual against its plugin, and every scene of every
plugin drawn with no error.

```sh
npm run shots -- tape-bloom        # docs/images/*.png from scenes.json
npm run docs -- tape-bloom --pdf   # the manual, checked and printed
npm run bundle -- tape-bloom --install
cargo run -p tape-bloom --example tape-bloom-demo
```

`bundle` builds the plugin in release and lays it out under
`target/bundle/`; `--install` also copies it to your own plugin
folders, with no admin.

## Release

A plugin's version is its crate's, and a version is released under
the tag `NAME-vVERSION`. Whenever main changes, CI builds every plugin
whose version has no tag yet and makes a GitHub release of it: a zip
with the VST3, the CLAP, the manual and the demos. A pull request
labelled `daily` is merged by CI as soon as the check passes, so a
daily plugin goes from pull request to release with no one in between
(`docs/decisions/0003`).

The plugins are not code signed.

## Credits

The kit was extracted from [wispidi](https://github.com/doesdev/wispidi).

## License

MIT © [Andrew Carpenter](https://github.com/doesdev)
