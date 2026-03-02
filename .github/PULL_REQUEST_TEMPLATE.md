## Summary

<!-- One sentence: what does this PR do and why? -->

Closes #

## Type of change

- [ ] `fix:` — bug fix (non-breaking)
- [ ] `feat:` — new feature (non-breaking)
- [ ] `feat!:` / `fix!:` — **breaking change**
- [ ] `refactor:` — internal restructure, no behaviour change
- [ ] `docs:` — documentation only
- [ ] `test:` — tests only
- [ ] `chore:` — build, CI, dependencies

> The PR title must follow [Conventional Commits](https://www.conventionalcommits.org/) —
> it becomes the commit message when squash-merged and feeds the auto-generated CHANGELOG.

## Changes

Brief bullet list of what changed and why.

## Testing

- [ ] `cargo fmt --check` passes
- [ ] `cargo clippy -- -D warnings` passes
- [ ] `cargo test` passes (all existing + new tests)
- [ ] Manual verification with a `~/.susshi.yml` (describe scenario below if relevant)

<!-- Optional: describe manual testing scenario -->

## Screenshots / recordings

<!-- If this changes the TUI, add a screenshot or asciinema recording -->

## Notes

<!-- Breaking changes, migration path, open questions, etc. -->

> ⚠️ Do **not** bump the version in `Cargo.toml` — this is handled automatically by release-plz when this PR is merged.
