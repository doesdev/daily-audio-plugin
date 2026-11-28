# Undertow: what changed

## 0.5.4 (2026-11-28)

- Fixed: as a VST3 it told the host of its meters every block, which
  at small audio buffers is a load on the host and can break up the
  sound. They are told thirty times a second now.

## 0.5.3 (2026-11-28)

- When linking, other instances are listed by the track they are on,
  as in "Drum Bus, Undertow", where the host tells a plugin its track.

## 0.5.2 (2026-11-27)

- The DAP mark is at the top left of the editor, with the name under
  it. The line that says which half leans on which now shows when the
  name is pointed at.

## 0.5.1 (2026-11-27)

- Hosts file it under Distortion. In Cubase it was under "Other".

## 0.5.0 (2026-11-27)

- Any setting can be linked to the same setting in other DAP
  instances: right-click it. Right-click the power button to link
  Bypass, or the Tube button to link the whole tube window.
- Fixed: in some hosts every setting went back to its default when a
  project was reopened.
- Fixed: a right-click on an empty part of the editor no longer opens
  a browser menu.
- The maker shows in hosts as Daily Audio Plugin.

## 0.4.0 (2026-11-26)

- The tube window gains Stage (Triode, Strapped, Pentode), Bloom and
  Feedback, and characters: settings of the whole window after a kind
  of valve stage. At rest it sounds as it did.

## 0.3.0 (2026-11-26)

- A sidechain input and a Sidechain switch: another track's low end
  can do the pushing.

## 0.2.0 (2026-11-26)

- Listen (Both, Mid, Side) to hear either half alone, and a Balance
  between them.
- A tube window on the valve itself: Bias, Grid, Sag and Glow.
- Nothing in the editor moves when it is pointed at, hints are drawn
  by the editor and not the browser, and the editor fills its window.
- Meters fall at a rate chosen in Settings.
- Fixed: Follow moves across its whole range when dragged or
  scrolled.

## 0.1.1 (2026-11-26)

- A Mix, and level matching.
- Fixed: the release zip opens in Windows' own zip handling.

## 0.1.0 (2026-11-26)

- First release: the middle's low end biases the valve the sides go
  through, or the other way round.
