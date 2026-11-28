# Hand-Me-Down: what changed

## 0.1.2 (2026-11-28)

- Fixed: dropouts at small audio buffers. As a VST3 it told the host
  of its meters every block, thousands of times a second at a buffer
  of 32, and it cleared eight seconds of memory on the audio thread
  whenever the host reset it, as when the transport starts.

## 0.1.1 (2026-11-28)

- When linking, other instances are listed by the track they are on,
  as in "Drum Bus, Undertow", where the host tells a plugin its track.

## 0.1.0 (2026-11-27)

- First release: each note keeps its own hit and plays the body an
  earlier note left behind.
