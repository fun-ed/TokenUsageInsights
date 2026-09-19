# Core map

- The root `AGENTS.md` is the authoritative repository guide for architecture, commands, testing, release, and contribution rules.
- Read `mem:backend/core` for API/data-flow, blocking, persistence, and transcript-security invariants.
- Read `mem:frontend/core` for dashboard state, API, UI-text, and assistant-metadata invariants.
- Read `mem:tech_stack` for runtime/package/release-layout facts; `mem:suggested_commands` for execution; `mem:conventions` for coding patterns; and `mem:task_completion` for verification gates.
- Keep local databases, session logs, and personal paths out of Git; use source-root environment overrides for isolated work.