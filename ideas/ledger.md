# Ledger

Every plugin made and every day that did not make one, newest last.
Append only. The verdict is a person's, after listening to the demos:
it is the one thing here a run cannot fill in, and it is what steers
which ideas come next.

| Date | Plugin | The idea, in a sentence | Outcome | Verdict |
|------|--------|-------------------------|---------|---------|
| 2026-11-25 | (kit) | The kit, extracted from wispidi, and the template plugin | Built; CI green on Windows, both validators pass | |
| 2026-11-25 | undertow | The middle's low end biases the sides' valve, sample by sample, so the sides take on even harmonics in time with the bass while the middle and mono pass untouched | Built; check green on Linux; Windows editor and validators left to CI | |
| 2026-11-26 | (kit) | Repair: the undertow 0.1.0 zip opened empty on Windows, as bsdtar wrote every entry as ./NAME; zips now name their files, and release notes keep the README's first paragraph whole. Undertow re-released as 0.1.1 | Fixed; check green on Linux; Windows unzip not seen here | |
| 2026-11-26 | aftersound | Every note is two coupled strings, one in each channel, so the in-phase mode the bridge drinks fast is the middle and the out-of-phase mode that rings on is the side: a note starts narrow and blooms wide as it dies | Built; check green on Linux; Windows editor and validators left to CI. Also: the check runs a plugin's own page tests, and a fitted number keeps its line level | |
| 2026-11-27 | hand-me-down | Each note keeps its own hit and then plays the body an earlier note (one to four back) left, from the same point in that note's life, into both halves, the middle or the sides: the pluck is now and the ring is one note late | Built; check green on Linux; Windows editor and validators left to CI | |
| 2026-11-28 | ironside | The sides are wound on a transformer core the middle fills, a saturable reactor: as the middle's flux fills the core the sides' choke gives way and their low corner climbs on each swing of the bass, so a low note clears the sides far sooner than a high one | Built; check green on Linux; Windows editor and validators left to CI. Kit: a core can be read by a second winding (`Transformer::opening`), the transformer's Size now starts exactly at its default, and (after clap-validator refused Ceiling on the first Windows run) only whole-number settings are stepped to a CLAP host. Found and not fixed: a value whose number and unit only just overflow its column wraps the unit ("3.00 kHz" in seven columns; Ironside shows one decimal to step round it), and the Settings panel at the default window lets the glass's meters show through the header in the shot (Undertow's `quality.png` has it too; Ironside has no such scene) | |
