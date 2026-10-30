# AGENTS.md

The canonical agent instructions for this repository are in [AGENT.md](AGENT.md). Read it before changing anything. The short version:

- **The code is the source of truth.** Delivered behavior is defined by `src/` (TypeScript) and `src-tauri/` (Rust); do not create or update user stories for implemented work. `userstory/` is only for future proposals.
- **Code first; documentation last.** Do not edit `README.md`, `docs/`, `docs-site/`, `CHANGELOG.md`, design-system prose or future proposals until the user has tested the local implementation and given an explicit OK. Implement, ask them to test, and wait.
- **Commits are in English** with Conventional Commits; use the `caveman-commit` skill when committing. Never commit secrets.
- **The design system is for new visual work.** Composition-only changes that reuse or remove existing components can go directly to the app with the user's OK; do not create a DS page for them.
- **Verify before finishing:** frontend `pnpm test` and `pnpm build`, design system `pnpm ds:check`, backend `cargo test` and `cargo clippy --all-targets -- -D warnings`.
