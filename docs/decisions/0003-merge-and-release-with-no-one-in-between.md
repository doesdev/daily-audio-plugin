# 0003: A daily plugin is merged and released with no one in between

2026-11-25

A pull request labelled `daily` is merged by CI as soon as the check
and both validators pass on Windows, and every plugin on main whose version has no tag
is then built and released. Nobody reviews or listens first. That was
the owner's choice.

What stands between an idea and a public release, then, is only what
is automatic:

- the checks in `kit/test`, each proven to fail a plugin that breaks
  it (`kit/test/tests/teeth.rs`);
- the plugin's own tests, written before its sound;
- the manual checked against the plugin, and every scene drawn;
- a `NOVELTY.md` with something under each heading;
- the rule that no check is weakened to pass a plugin
  (`docs/daily-run.md`).

What nothing automatic covers: whether it sounds good, whether the
novelty claim is true, and whether a host this repo does not test
loads it. The ledger's verdict column is where a person records the
first of these after the fact. A release that should not have gone out
is withdrawn by deleting its GitHub release; its tag stays, so the
version is not reused.

The merge is a job of the check workflow, not GitHub's auto-merge, so
it needs no branch protection to be safe: it only runs after `check`
succeeds. A merge made with the workflow's token starts no further
workflow, so the release is a job of the same run.

To put a person back in: remove the `merge` job, or stop applying the
label.
