---
description: Refresh the code-index memory files (architecture, components, API, forms) from the current src/ tree
---

Refresh the four code-index memory files so they match the current state of `src/`. These live in the auto-memory dir:
`/Users/maxim/.claude/projects/-Users-maxim-Projects-Rust-gotiga/memory/`
 — `project-architecture.md`, `component-map.md`, `api-methods.md`, `forms-inventory.md`.

Goal: an **architectural map**, not exhaustive dumps. One line of purpose per file + relations (who renders/calls what) + where to look. Keep them token-lean — they auto-load every session. Detail beyond a one-liner stays just-in-time (the reader greps/reads the file when needed).

## Steps
1. **Survey the tree** (don't read every file fully — extract signals):
   - `find src -type f \( -name "*.svelte" -o -name "*.ts" \) | sort` — full file list
   - Routes + loaders: grep `api\.` in each `+page.ts`/`+layout.ts`
   - api.ts methods: grep `^\s*async [a-zA-Z]` in `src/lib/api.ts`
   - Stores: grep `export (const|class|function)` in `src/lib/stores/*.ts`
   - Component props: perl-extract the `let { … } = $props()` block per `.svelte`
   - Relations: grep `from '\$lib/components/` in route pages + container components
   - For genuinely new/ambiguous components, read the leading `/** … */` doc-comment only
2. **Diff against the existing four files** — what's new, renamed, deleted, or changed. Preserve any hand-written `⚠️`/gotcha notes that are still true; drop ones that are now false.
3. **Rewrite** each of the four files via Write. Keep the frontmatter (`name`, `description`, `metadata`) intact. Update the `_Last refreshed: YYYY-MM-DD_` line.
4. **Update `MEMORY.md`** — bump the "last full refresh" date in the code-index note; fix any pointer descriptions whose scope changed.
5. **Report** a short changelog: files added/removed since last refresh, and any new subsystem worth a dedicated memory.

## Rules
- Never duplicate these indexes inside `src/` — single source of truth is the memory dir.
- Architectural depth only. If you're tempted to paste full method bodies or every field of every form, stop — link to the file instead.
- Link related memories with `[[slug]]` (e.g. `[[design-system]]`, `[[display-config-feature]]`).
- Cross-check before asserting `⚠️ orphaned`/`dead` — grep for real usage first.
