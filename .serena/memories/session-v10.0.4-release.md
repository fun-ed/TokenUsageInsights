---
scope: project:token-usage-insights
project: token-usage-insights
jira: N/A
wiki: N/A
github_owner: fun-ed
github_repo: TokenUsageInsights
repository: fun-ed/TokenUsageInsights
folder: .serena/memories
occurred_start: 2026-09-19T00:00:00Z
occurred_end: 2026-09-19T19:18:49Z
device_label: macos-arm64-local
harness: other
cli: codex-cli
model: gpt-5.6-terra
effort: high
session_name: v10.0.4 manual release and local install
---

# v10.0.4 manual release

## Completed

- Version 10.0.4 was synchronized in Cargo and npm metadata.
- GitHub Actions workflows were removed. Releases now use local verification and manual GitHub uploads.
- The public `v10.0.4` release contains an Apple Silicon macOS DMG and `SHA256SUMS`. The uploaded checksum was downloaded and verified against the DMG.
- The root README is the concise canonical English document. Short Traditional Chinese, Simplified Chinese, Japanese, and Korean summaries remain.
- `make install-local` builds the release binary and installs it to `~/.local/bin/token-usage-insights`. The installed binary reports version 10.0.4.
- The project skill documents the manual macOS DMG build, installer smoke test, checksum, tag, and GitHub Release procedure.

## Follow-up

- `npx` and `scripts/get.sh` download target-specific `.tar.gz` assets. The v10.0.4 release currently has only a DMG, so those installer paths require compatible tarballs and matching checksum entries before they can be advertised as working.
- The `v10.0.4` tag points to commit `50b98d3`. Later documentation and Makefile commits are on `main` but are not part of that tag.
