# Release checklist (v1)

1. [ ] `docs/METRICS.md` rows 1–16 green
2. [ ] `cargo fmt --all -- --check`
3. [ ] `cargo clippy -- -D warnings`
4. [ ] `cargo test` (default members)
5. [ ] CI green on `main` (ubuntu + windows)
6. [ ] `cadrion skills export --all -o dist/skills --json`
7. [ ] `docs/LICENSING.md` current; optional `cargo license` artifact
8. [ ] Tag `v0.1.0` (or agreed version); dual-license headers present
9. [ ] README quick-tests still match CLI surface
10. [ ] No secrets in tree (`.env` gitignored)

Optional OCCT release notes: document LGPL binary obligations separately.
