# 0001: An owned kit, extracted from wispidi

2026-11-25

Every plugin here is built on a kit this repo owns, extracted from
wispidi: raw `vst3` and `clap-sys` bindings, the editor as a page in
the system webview, and the checks wispidi's plugins were held to.

Why not a third-party framework: making a plugin a day unattended is
limited by how well the result can be verified, not by how fast the
format glue can be written. wispidi already had a spec that drives
both formats and the editor, a page that can be drawn and driven in a
headless browser, a realtime budget tested through each format's own
front door, and docs checked against the code. None of the Rust
frameworks looked at (NIH-plug, in maintenance; its successor
nice-plug; truce) offers that for the editor.

What it costs: host compatibility is ours to keep. pluginval and
clap-validator in CI are the guard (0004).

What changed in the extraction: wispidi's glue knew wispidi. Here a
plugin is one trait (`kit::Plugin`) and everything else is generic
over it or reads its `Spec`. wispidi itself was left alone and is not
on the kit.
