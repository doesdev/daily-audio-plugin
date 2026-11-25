# 0002: One crate a plugin, one library for both formats

2026-11-25

A plugin is one crate. Its library exports both the VST3 entry points
and `clap_entry`; `npm run bundle` copies the same file to
`NAME.vst3/Contents/x86_64-win/NAME.vst3` and to `NAME.clap`.

wispidi has a crate a format, because each format's side was written
by hand. Here both are a macro each (`kit_vst3::export!`,
`kit_clap::export!`), so a second crate would only be a second place
for the two to drift apart. One library also means the two formats
cannot differ in sound: the check `loads_in_every_format` compares
each against the plugin run directly.

A plugin's version is its crate's version, and it is released under
the tag `NAME-vVERSION`, so plugins release independently.
