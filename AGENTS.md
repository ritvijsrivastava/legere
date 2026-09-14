# AGENTS.md

Minimal ground rules for working in this repo. Read the linked docs
when they're relevant to the task at hand — don't duplicate their
content here.

## Rules

- Run from the **repo root**, not `frontend/` — Tauri CLI commands
  resolve `tauri.conf.json` (in `src-tauri/`) relative to the working
  directory.
- Before building/testing, see **README.md** for desktop/Android dev
  commands and prerequisites (Android SDK/NDK, JDK 17/21, `jq`).
- Before touching capture/schema/event internals or planning a larger
  change, see **ARCHITECTURE.md** — it documents the architecture,
  locked design decisions, and the history behind superseded ones
  (e.g. the removed ZIM/"Original" archive viewer). If the change
  alters any of that — architecture, data flow, schema, or a locked
  decision — update **ARCHITECTURE.md** in the same change, not as a
  follow-up.
- Before making product-scope or UX-copy decisions, see **PRODUCT.md**
  (positioning, principles, capabilities/constraints) and
  **DESIGN.md** (colors, typography, components, do's/don'ts) — new UI
  must follow the existing design system, not invent a new one.
- Before cutting a release or touching CI/signing, see
  **docs/RELEASING.md**.
- Offline-first constraint: no feature may assume network reachability
  or load remote fonts/icons/assets; vendor everything.
- Run the test/lint commands in README.md's "Testing" section before
  considering a change done: `cargo test`, `cargo clippy`, and
  `svelte-check`.
- After any change, update all related docs and `.md` files
  (README.md, ARCHITECTURE.md, PRODUCT.md, DESIGN.md,
  docs/RELEASING.md, this file) so they stay accurate — don't leave
  documentation stale.

## Git commit messages

- Imperative, present-tense summary line (e.g. "Add X", "Fix Y",
  "Replace Z"), no trailing period.
- Optional body paragraph(s) below a blank line explaining what
  changed and why, in plain prose; use a short bullet list only for a
  trailing list of smaller/unrelated fixes bundled in the same commit.
- No `Co-authored-by`, no AI/tool attribution footers, no issue-tracker
  boilerplate — just the summary and (optional) body.
