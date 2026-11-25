# The design canvas

`canvas/` is the source of the design canvas the house look was
settled on (`docs/decisions/0007-direction-c.md`): one `.dc.html` file
a board, 820 by 500, and `canvas.json`, which lays them out. Boards A
(`Main`) and B (`Columns`) are the two directions not taken. Every
other board is direction C, light and dark.

Each board is a template: the markup inside `<x-dc>` with `{{ name }}`
holes and `sc-for` loops, and a `renderVals()` in the script at the
foot that fills them. They are plain enough to read as they are. The
measures, colours and wording in them are the specification for every
control `docs/design.md` lists as drawn and not yet built.

A control is built from its board, not redesigned. If a plugin needs
something no board has, it is drawn here first, in the same hand, in
both themes, and looked at before it is built.
