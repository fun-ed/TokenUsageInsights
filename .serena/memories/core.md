# Core map

- `docs/fork-spec.md` is the canonical product/maintenance spec; root `AGENTS.md` is the repository guide for architecture, commands, testing, release, and contribution rules. Spec wins on conflicts.
- Read `mem:fork_spec` before feature, release, upstream-sync, profile-discovery, or service-install work: platform scope, sync removal checklist, Claude profiles, auto-import, retention, known Windows/updater residue.
- Read `mem:backend/core` for API/data-flow, blocking, persistence, and transcript-security invariants.
- Read `mem:frontend/core` for dashboard state, API, UI-text, and assistant-metadata invariants.
- Read `mem:tech_stack` for runtime/package/release-layout facts; `mem:suggested_commands` for execution; `mem:conventions` for coding patterns; `mem:task_completion` for verification gates; `mem:manual_release_and_readme` for release/README state.
- Keep local databases, session logs, and personal paths out of Git; use source-root environment overrides for isolated work.
