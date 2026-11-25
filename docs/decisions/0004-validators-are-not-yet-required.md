# 0004: pluginval and clap-validator run, but do not yet gate

2026-11-25

The `validate` job bundles every plugin and runs pluginval on the VST3
and clap-validator on the CLAP. It is `continue-on-error`: a failure
shows on the run but blocks neither the merge nor the release.

Why not required from the start: the job was written where neither
tool nor Windows could be run, so its first runs test the job as much
as the plugins. The validators' download names are matched by pattern
and may be wrong.

Make it required once it has been seen green on the template: remove
`continue-on-error` and add `validate` to the `needs` of `merge` and
`release`. That is the point at which 0001's guard is really in place.

## Since

2026-11-25, the same day: the job ran green on the template (pluginval
at strictness 5 passed; clap-validator ran 44 tests, 32 passed, none
failed, 12 skipped), so it is now required. `continue-on-error` is
gone and `merge` and `release` both need `validate`.
