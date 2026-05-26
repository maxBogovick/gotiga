# Media & Content Implementation Progress

Date: 2026-05-26

## Scope

This file tracks the concrete media/content fixes implemented during this pass.
It is intentionally factual: each item should end as Done, Partial, or Not Done,
with the verification command or reason.

## Checklist

- [x] Server `/upload` returns the contract expected by Tauri and web clients.
  - Implemented: response now includes both `url` and `relativePath`.
- [x] Tauri `push_figurine` handles upload failures without panics.
  - Implemented: removed `unwrap()` and propagates upload errors.
- [x] Tauri local figurine delete removes the database record instead of no-op.
  - Implemented: added `delete_figurine` command and repository method.
- [x] Media import/upload validates supported file types.
  - Implemented: Tauri import and server upload validate extensions by media bucket.
- [x] Release export verifies referenced media are available as files or existing BLOBs.
  - Implemented: export aborts with a missing-media report.
- [x] Release export includes app resources such as main background in BLOB payload.
  - Implemented: app resources are packed and verified, except JSON-only author profile.
- [x] Media removal/orphan cleanup path exists.
  - Implemented: `cleanup_unused_media` Tauri command and admin Releases button.
- [x] Checks pass: `npm run check`, `cargo check` for Tauri, `cargo check` for server.
  - Verified: `npm run check` passes with 0 errors and 0 warnings.
  - Verified: `cargo check` for Tauri passes with 0 crate warnings.
  - Verified: `cargo check` for server passes with 0 crate warnings.
  - Verified: `cargo test` for Tauri passes: 2 media helper tests.
  - Verified: `cargo test` for server passes: 2 media helper tests; 1 full release
    integration test is ignored by default because it requires a reachable PostgreSQL
    test database.
- [x] Full image optimization/resizing.
  - Implemented: Tauri image import decodes supported images and writes JPEG variants:
    `images/original/<id>.jpg`, `images/preview/<id>.jpg`, `images/thumb/<id>.jpg`.
  - Implemented: server web upload also decodes images and writes the same JPEG variants
    under `/static/images/original`, `/static/images/preview`, and `/static/images/thumb`.
  - Implemented: server upload response keeps the old `url`/`relativePath` fields and adds
    `originalUrl`, `originalRelativePath`, `thumbUrl`, and `thumbRelativePath`.
  - Implemented: SQLite schema now stores `original_path`, `thumb_path`,
    `original_data`, and `thumb_data`.
  - Implemented: release export embeds preview/original/thumb BLOBs and verifies all
    referenced image variants before upload.
  - Implemented: Tauri and server DTOs expose `originalUrl` and `thumbUrl`; list/related
    cover images use thumbnails when available, detail view uses preview, lightbox uses
    original.
- [x] Admin Media Library for real media operations.
  - Implemented: new admin `Media` tab with all managed files, sizes, existence state,
    preview/original/thumb variant labels, usage list, orphan filter, missing-file filter,
    cleanup dry-run report, protected cleanup, and replace-everywhere action.
  - Implemented: Tauri commands `get_media_inventory`, `get_unused_media_report`,
    `cleanup_reported_unused_media`, and `replace_media_everywhere`.
  - Implemented: server endpoints under protected `/api/v1/admin/media`,
    `/api/v1/admin/media/cleanup-report`, `/api/v1/admin/media/cleanup`, and
    `/api/v1/admin/media/replace`.
  - Implemented: cleanup only deletes files with zero references in the latest inventory.
    Used files are not deleted by the Media Library cleanup flow.
  - Implemented: replace-everywhere imports the replacement file, updates all matching DB
    references, and for image records replaces the full preview/original/thumb set.
  - Verified: `npm run check`, Tauri `cargo check`, server `cargo check`, Tauri
    `cargo test`, and server `cargo test` pass.

## Decisions

- MVP source of truth remains the local SQLite content database.
- Publishing should prefer the portable SQLite release with embedded BLOB media.
- Direct per-figurine upload remains supported, but it is not the primary release path.

## Notes

- Physical deletion is intentionally exposed as explicit orphan cleanup, not automatic
  deletion on every remove click.
- Media Library cleanup uses a report-before-delete flow. The delete step recomputes
  orphans before deletion, so a file that became referenced after the report is protected.
- `images/preview` remains the canonical `url`; `originalUrl` and `thumbUrl` are additive
  optional fields for clients that can use image variants.
- Svelte diagnostics were cleaned after the media work; remaining Cargo note is a dependency
  future-compatibility notice from `sqlx-postgres`, not a crate warning.
- `image` dependencies are restricted to `jpeg`, `png`, and `webp` features to avoid
  pulling unnecessary image codecs into normal builds.
