# The daily run

One run makes one plugin, start to finish, and leaves the repo able to
make the next. A run starts with no memory of the last: everything it
needs to know is in this repo, and everything the next run needs to
know must be written back to it before the run ends.

## The promise, and its limits

Each plugin does something new to the world, not just new to this
repo. Nobody can prove that. What a run can do is look hard, write
down where it looked and what it found, and say plainly what the
nearest existing things are. `NOVELTY.md` is that record. A run that
finds its idea already exists drops the idea; it does not reword it
until it sounds new.

A run cannot hear. The checks catch a plugin that is broken, slow or
silent; they do not catch one that is dull. The demos in each release
are how a person finds that out, and the ledger is where they say so.

## 1. Read

- `ideas/ledger.md`: every plugin made and every idea dropped, so
  nothing is made twice.
- `ideas/backlog.md`: ideas waiting, and notes on what kinds have
  gone well.
- `docs/kit.md`: how a plugin is written, and what the kit cannot do
  yet.

## 2. Repair first

Look at the last check run on main and at yesterday's pull request.
The run happens on Linux; the editor only builds on Windows, in CI.
So yesterday's Windows result is the first thing today's run learns.

- Main is red: fix that and nothing else today. Open a pull request
  with the `daily` label for the fix, add a ledger line, and stop.
- Yesterday's pull request is red or unmerged: fix it on its branch.
  If that takes the day, that was the day.

## 3. Choose the idea

Take the first idea in the backlog that still holds up, or think of a
better one. A good idea:

- is one idea, said in a sentence, that a musician would want to try;
- fits one screen and about a dozen settings at most;
- can be heard: a demo of it sounds different from a demo without it;
- can be built on the kit as it is, or with one small kit addition.

Then look for it, before writing any code:

- plugin databases and shops (KVR, Plugin Boutique, the big makers'
  catalogues), by what it does, not what you would call it;
- GitHub and the free plugin world (Airwindows, Surge, Vital, Valhalla
  and the like);
- papers: DAFx, AES, ICMC, arXiv;
- patents, by technique (Google Patents).

Write `NOVELTY.md` (`npm run new` makes its outline): what is new, the
nearest existing things with links and how this differs from each,
where you looked, and what would change the answer.

- It already exists: write it in the backlog under Dropped with the
  link, and choose again.
- A patent that looks in force seems to cover the technique: do not
  build it. Write it under Dropped with the patent's number. This is
  not legal advice and a run is not a lawyer; when unsure, drop it.
- It exists but this takes it somewhere it has not been: say exactly
  where, in the first section. "X, but with Y" is fine when Y is real.

## 4. Build, in this order

```sh
npm run new -- NAME "Display Name"      # --instrument if it is played
```

1. `NOVELTY.md`.
2. The spec in `src/lib.rs`.
3. `tests/sound.rs`: what it promises, failing.
4. The sound, until `cargo test -p NAME` passes.
5. The editor: `ui/main.js`, `ui/style.css`. It should show what the
   plugin does, not just list its settings.
6. `docs/scenes.json`, `npm run shots -- NAME`, and look at every
   picture. A picture that looks wrong is a bug.
7. `docs/manual.md` and `README.md`.
8. `examples/demo.rs`: the settings that show what is new.
9. `npm run check`.

## 5. The rules that keep this honest

- Never weaken a check to pass a plugin. Not a threshold, not an
  `#[ignore]`, not a deleted assertion. If a check is wrong, that is a
  kit change with its own reasoning in `docs/decisions/`, in its own
  pull request, and no plugin ships on it the same day.
- A kit change goes in its own commit with its own tests, and the
  template must still pass.
- Never change an id that has shipped: not a setting's, not a class
  id, not the CLAP id.
- Never commit a picture you have not looked at.
- Say what was not verified. On Linux that is always the editor in a
  real WebView2 and both validators.
- No em or en dashes in docs. The check enforces it.

## 6. Hand it over

Branch `daily/YYYY-MM-DD-NAME`. Commit the kit changes first, then the
plugin. Add a line to `ideas/ledger.md` and move the idea out of the
backlog in the same pull request.

Open a pull request with the `daily` label. Its body says:

- what the plugin does, in two sentences;
- what is new about it, from `NOVELTY.md`, and the nearest thing found;
- the editor's picture;
- what was run and passed, and what could not be run here.

CI then runs the check on Windows. If it passes, CI merges the pull
request and releases the plugin. If it fails, the pull request stays
open and tomorrow's run starts there.

## 7. When the day does not work out

A run that cannot get a plugin through the check does not ship one.
Push the branch, open the pull request as a draft without the `daily`
label, and write the ledger line with what went wrong and what would
fix it. A day with an honest "not today" is worth more than a plugin
that should not have shipped.

If the same thing in the kit got in the way twice, the next run is a
kit day: fix it, with tests, and make no plugin.
