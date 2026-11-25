# 0006: One house look, monotone with purple for the value

2026-11-25

Every editor starts from one look, set out in `docs/design.md`: an
off-white or dark grey ground, purple as the only accent and only for
a setting's value, other colours only where they already mean
something (a level meter's zones, a control's `tone`), a light and a
dark theme, and one typeface carried in the page.

The first version of the kit's page was what an agent draws by
default: dark cards with borders, capital letter-spaced labels,
monospace values and a teal accent. A plugin a day in that look would
be a catalogue of the same generated page. The owner asked for a look
that avoids those habits; `docs/design.md` lists them so a daily run
does not drift back.

Archivo is embedded as a data URL in `kit/ui/fonts.css` (about 90 kB a
plugin). The page has no network and a host's machine may have any
fonts or none, so a font that is not carried is a font that changes.
It is under the SIL Open Font License (`kit/ui/fonts/OFL.txt`), which
allows shipping it in a product.

The theme is kept in the page's own storage, which every editor of
ours in one host shares, so choosing it once chooses it for all. It is
not a plugin setting: a host does not automate it or save it with a
project.

## Since

2026-11-25, later the same day. The owner found the first house look
still generic: flat, with a knob as a ring of lit ticks and a meter in
green, amber and red, both of them the stock answers. Two things were
added to the brief: the editor must meet or exceed what the best
makers in the field ship, in use and in character, without copying
them; and the look should have depth.

So the look was rebuilt from what those makers get right rather than
from how plugins usually look (`docs/design.md`, "The bar"):

- a screen every editor is built round, for a picture only that
  plugin has;
- the rule, a setting on one line, as the house control instead of a
  bank of knobs;
- a meter in ink with a held peak and a clip lamp;
- three depths with light from above, where standing off means "you
  can take hold of this";
- undo, redo and A/B compare in every header, typed values, a line at
  the foot that says what a control does, and a page that scales
  whole to its window.

What is still under the bar is written down in the same place. The
largest is that no live signal reaches the page.
