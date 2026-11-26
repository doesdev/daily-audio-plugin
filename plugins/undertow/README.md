# Undertow

The sides of a stereo sound go through a valve-like curve, and the low
end of the middle decides, sample by sample, where on that curve they
sit. The sides take on even harmonics and give a little ground in time
with the bass itself; the middle, and any mono sound, pass untouched.

![The editor](docs/images/editor.png)

- `NOVELTY.md`: what is new about it, and what was found looking for
  it elsewhere.
- `benchmark.json`: how it stands against the best of its kind.
- `src/lib.rs`: the spec (who it is, its settings and its meters) and
  the sound.
- `ui/main.js`, `ui/style.css`: the editor, built round the curve.
- `tests/suite.rs`: the checks every plugin is held to.
- `tests/sound.rs`: what this plugin itself promises.
- `docs/manual.md`, `docs/scenes.json`, `docs/images/`: the manual and
  the scenes its pictures are taken from.
- `examples/demo.rs`: renders WAV demos to listen to.
- `examples/spec.rs`: the spec as JSON, for the docs tools.
