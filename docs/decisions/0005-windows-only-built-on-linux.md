# 0005: Windows only, made on Linux

2026-11-25

Plugins are released for Windows 11 x64 only, as wispidi is. The daily
run happens in a Linux container.

Checked on Linux, by `npm run check`: the kit and every plugin through
`Runner`, through the VST3 factory and through the CLAP entry (state,
parameters, audio, MIDI, allocation, budget); the page in a headless
browser over a stand-in host; every scene; every manual.

Only checked on Windows, in CI: that the Windows-only code compiles at
all (`kit-webui`'s panel and window, the VST3 editor, the CLAP GUI),
the editor in a real WebView2 hosted as a DAW hosts it, and both
validators.

So a pull request can pass here and fail there. The daily run's first
step is to look at yesterday's CI for exactly that reason.

The SDK's enums are `i32` on Windows and `u32` elsewhere; `kit-vst3`
casts them at each use so the same code builds on both.

Other platforms later: the audio side already builds anywhere. The
editor needs `window.rs` and the `HWND` checks done for each platform;
`wry` already speaks WKWebView and WebKitGTK.
