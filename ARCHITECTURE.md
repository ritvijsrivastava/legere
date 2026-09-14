# Architecture

This document explains how Legere works and why it is built the way it is —
the structural decisions, the invariants that other code depends on, and the
history behind the choices that look unusual. Code comments carry the
line-level detail; this carries the shape of the system.

Related reading: [PRODUCT.md](PRODUCT.md) (what the product is and isn't),
[DESIGN.md](DESIGN.md) (the design system), [docs/RELEASING.md](docs/RELEASING.md)
(releases and signing).

## Design principles

These are the locked decisions everything else follows:

1. **Offline-first.** No feature may assume network reachability. Fonts,
   icons, and every other asset ship vendored with the app; captured articles
   keep their readable view and its own images on disk forever.
2. **Local-first, no server.** One SQLite database per device, no accounts,
   no sync. The schema is sync-friendly (UUID primary keys, `updated_at` on
   every row) so sync could be added later, but nothing assumes it.
3. **One capture pipeline.** RSS entries, pasted links, Raindrop import rows,
   and re-captures all converge on a single
   `capture::capture_local` fetch→extract→sanitize→localize→store pipeline.
   There is exactly one place where "a URL becomes an article".
4. **Narrow, honest scope.** Legere makes the *readable view* durable; the
   original page is never archived or rendered in-app. "View original" is a
   plain external link to the cleaned URL.
5. **Sanitize at capture, lock down at render.** Stored HTML is stripped of
   every script-execution vector at capture time, and the webview's CSP
   refuses scripts anyway. Two independent layers, either of which suffices.

## Stack and layout

```
frontend/          SvelteKit (Svelte 5 runes, static adapter, TypeScript)
src-tauri/         Rust backend (Tauri 2)
  src/
    capture/       the shared capture pipeline (see below)
    sources/       RSS sync, direct links, Raindrop CSV import
    db/            pool, migrations, queries
    commands/      Tauri command handlers (thin wrappers)
    content_server.rs  legere-content:// protocol handler
    events.rs      backend→frontend event emission
    gc.rs          startup orphan-file sweep
    sync.rs        autosync loop
    urlx.rs        URL canonicalization / tracking-strip / path mapping
    state.rs, error.rs, models.rs, mobile_tls.rs
plugins/           tauri-plugin-apk-installer (Android self-update, optional feature)
.github/           release + cache-warming workflows
```

Storage lives at `{app_local_data_dir}/legere/`:

```
legere.db          SQLite (WAL, foreign keys ON, busy_timeout 5s, pool of 4)
content/<id>/      per-article localized images (never evicted)
media/<id>.jpg     resized hero thumbnails for library cards
```

## The capture pipeline

Every ingestion path funnels into `capture::capture_local(client, data_dir, id, url)`:

1. **Fetch** (`capture::fetch`) — through the SSRF-guarded client (below),
   following redirects; the final URL is the base for all later resolution.
2. **Extract** (`capture::extract`) — `dom_smoothie` produces the readable
   fragment, title, excerpt, publish date, and read time. If extraction
   fails confidence checks, a naive fallback stores sanitized body text with
   `extraction_confident = false`. `sanitize_title` rejects any title
   containing U+FFFD (proof the source was never proper UTF-8 — garbled
   titles once froze text shaping in the webview), folds control characters
   to spaces, and caps length at 300 chars.
3. **Sanitize** (`capture::sanitize`) — a streaming (`lol_html`) *denylist*
   pass stripping `<script>`, `on*` handlers, `javascript:`/`data:text/html`
   URIs, script preloads, dangerous `<meta refresh>`, `<base>`, and iframes
   (replaced with a placeholder link). Denylist rather than allowlist
   deliberately: the rewrite pass must still find `srcset`, `video[poster]`,
   tables, etc.
4. **Localize** (`capture::localize`) — discovers every `img[src]`,
   `img/source[srcset]`, `video[poster]` reference in the sanitized fragment,
   resolves it against the final URL, and fetches each asset (concurrency 6,
   20 MB per-asset cap, one retry with backoff). This is scoped to the
   readable fragment only — page-level CSS/JS is never fetched.
5. **Rewrite** (`capture::rewrite`) — rewrites the same attribute set to
   `legere-content:/<article_id>/<local_path>` tokens using the URL→path map
   localization built. Assets that failed to fetch are left pointing at their
   original remote URL — graceful degradation, not broken local links.
   Stored HTML stays platform-neutral; the frontend resolves tokens at
   render time (see below).
6. **Store** (`capture::archive`) — assets are written to
   `content/<article_id>/<local_path>` (directory wiped first, so recapture
   never leaves stale files); the hero image is a resized JPEG in `media/`,
   reusing already-fetched bytes when the hero also appears in the content.

The article's stored `link` is the canonicalized URL (fragment and redundant
default port dropped, query order preserved byte-for-byte) with tracking
parameters stripped (`urlx::strip_tracking_params` — a curated blocklist;
bare `ref`/`source` are deliberately *not* stripped, too many false
positives). `UNIQUE(link)` on the `articles` table makes dedup an enforced
invariant: duplicate inserts are `INSERT OR IGNORE` no-ops, never a racy
pre-check.

### URL → path mapping (`urlx::local_path_for`)

Asset URLs map deterministically to filesystem-safe paths:
`<scheme>/<host>[-portN]/<sanitized segments>`. Segments are
percent-decoded, control/unsafe characters replaced with `_`, `.`/`..`
neutralized, truncated to 120 bytes; a short FNV-1a hash suffix
disambiguates collisions (any query string, or lossy sanitization). The same
URL always lands at the same file, which is what lets the rewrite pass and
the protocol handler agree without a manifest.

### SSRF guard (`capture::ssrf`)

Every fetch the pipeline makes — page and assets — uses a client whose DNS
resolver drops loopback/private/link-local/CGNAT addresses. Two mechanisms,
both required (verified empirically):

- A custom `reqwest::dns::Resolve` impl filters resolved addresses at
  resolution time, which also closes the DNS-rebinding TOCTOU (the connected
  IP is exactly the checked IP).
- A `literal_ip_is_blocked` pre-check covers literal-IP URLs
  (`http://169.254.169.254/...`), which never invoke the configured resolver
  at all.

## Serving content: the `legere-content://` protocol

`content_server.rs` registers an asynchronous URI scheme protocol. Stored
HTML contains `legere-content:/<id>/<path>` tokens; the frontend
(`api.ts::resolveContentTokens`) replaces them with `convertFileSrc(...)`
URLs before `{@html}` rendering, which yields `legere-content://…` on desktop
and `http://legere-content.localhost/…` on Android.

The handler:

- parses the whole request path *after* percent-decoding (`convertFileSrc`
  encodes the joined path as one unit, so `/` may arrive as `%2F`);
- validates the article id against the database — this is the traversal
  guard, a forged id with files on disk still 404s;
- canonicalizes both sides of the path containment check (defends `..` even
  though `local_path_for` already prevents it, and handles symlinked data
  dirs like macOS `/var`);
- serves bytes with an extension-guessed MIME type and
  `Cache-Control: immutable` (content is write-once per capture).

## Ingestion paths

**RSS sync** (`sources::rss`) — fetch + `feed-rs` parse, backfill the
source's display name only while it's still the URL placeholder (never
overwrite a user rename), capture up to 30 new entries per sync
(`MAX_ENTRIES_PER_SYNC`), tagging from `<category>` elements. Errored
sources are retried every autosync cycle (no backoff) so they recover on
their own; paused sources are skipped.

**Add-a-source sniffing** (`commands::sources::add_source_auto`) — the user
pastes one URL with no up-front choice; the backend fetches it once and
tries to parse it as a feed. Parse success ⇒ recurring RSS source; anything
else (including parse failure) ⇒ one-shot direct-link capture. A new
direct-link article starts Uncategorized, moved later via its card or the
reader.

**Raindrop import** (`sources::raindrop_import`) — CSV rows run through the
same capture pipeline in a fixed-size worker pool (concurrency 5–10, a
clamped user setting). Design points:

- A read-only *preview* groups rows by folder with duplicate counts before
  anything is captured.
- Each folder resolves to a same-named category (created once per distinct
  folder up front — `find_or_create_category` isn't safe to race for the
  same new name); blank/`Unsorted` means Uncategorized.
- An already-saved link is always skipped outright, and its category is
  never changed by a re-import — only new tags are merged in.
- Per-row failures never abort the batch (a years-old export is full of dead
  links); progress flows through `import:*` events; an `AtomicBool` flag
  cancels dispatch of *new* rows while in-flight fetches finish.
- The import runs in the background and survives the dialog closing; the
  single-import guard is the same flag stored in `AppState`.

## Sync and lifecycle

- **Autosync** (`sync.rs`) — a foreground interval task (15 minutes) spawned
  at startup when enabled; toggling the setting spawns/aborts it. No
  WorkManager on Android — a resumed app syncs only if ≥5 minutes have
  passed since the last foreground sync (`RunEvent::Resumed` handler).
- **Events** (`events.rs` → `frontend/src/lib/events.ts`) — the backend
  emits `sync:started/finished`, `articles:changed`, `source:changed`,
  `category:changed`, and `import:*`. The frontend registers listeners once
  in the root layout and responds with coarse-grained store refetches, not
  payload-carried deltas — the dataset is small and refetch is what fixed
  the original "library never refreshes after autosync" bug.
- **State** (`state.rs`) — `AppState` holds the DB pool, the shared
  SSRF-guarded HTTP client, the data dir, autosync/cancellation handles,
  and (per-platform, cfg-gated) pending-update slots.

## Storage and schema

- **Pool** (`db::pool.rs`) — r2d2 over rusqlite, 4 connections. Init order
  matters: `busy_timeout` first (so WAL initialization on a fresh file waits
  instead of erroring), then `journal_mode = WAL`, then `foreign_keys = ON`.
- **Migrations** (`db::schema.rs`) — `rusqlite_migration`, currently V12.
  SQLite can't alter CHECK constraints, so schema-changing migrations use a
  recreate-repopulate-swap dance; foreign keys are toggled off around the
  whole migration (the pragma is a no-op inside a transaction). Each
  migration's doc comment records what it deliberately *didn't* backfill and
  why — several data-lossy collapses are accepted because the affected data
  was personal/pre-remodel, and those calls are documented rather than
  hidden.
- **Schema shape** — `sources` (RSS only since V2), `articles` (UUID PK,
  `UNIQUE(link)`, `reading_state` unread/reading/read, `reading_progress`
  0–1, `tags` JSON array, nullable `category_id`, per-article
  reading-appearance overrides, `updated_at` everywhere), `categories` (flat
  folders, case-insensitively unique names, `ON DELETE SET NULL` so deleting
  a category un-categorizes rather than deletes), `settings` (KV).
- **Tags** — always lowercase/trimmed/deduped; `db::queries::normalize_tags`
  is the single choke point every tag-writing path goes through.
- **Library queries** — keyset-paginated on `(fetched_at, id)` DESC
  (`list_articles_page`) with server-side search/category/tag/favorite
  filters, so the frontend never holds the whole table. Category/tag name
  matching for the search box is client-side against the sidebar's already-
  fully-fetched stats.
- **Reading state** — opening the reader transitions `unread`/`read` →
  `reading`; scroll progress is persisted debounced from the reader's scroll
  handler and restored on open. Per-article font/measure/leading/theme
  overrides are nullable columns; NULL means "follow the global setting".

## Garbage collection

- `delete_article` removes the row, then `content/<id>/` and the hero
  thumbnail; idempotent on a missing id.
- `delete_all_articles` wipes `content/` + `media/` wholesale (cheaper than
  per-row cleanup when everything is going); sources are kept.
- `gc::sweep_orphaned_files` runs once per startup, spawned so it never
  delays launch: any `media/` file or `content/` entry without a live
  article row is removed. This catches crash orphans and files left by
  pre-`delete_article` code paths (including stale pre-V6 `.zim` files).

## Security model

Stored article HTML is untrusted input rendered with `{@html}`, so:

1. **Sanitize at capture** (layer 1) — the denylist sanitizer strips every
   known script-execution vector before anything is stored; the sanitizer's
   tests include an adversarial-input case asserting zero execution vectors
   survive.
2. **CSP** (layer 2) — a real (non-null) content-security policy in
   `tauri.conf.json`: `script-src 'self'`, image/media sources limited to
   `self`, the asset protocol, `legere-content:` (and its Android
   `http://legere-content.localhost` form), and `data:`; `connect-src` is
   `self` + IPC only. Scripts never load from captured content.
3. **SSRF-guarded fetching** — see above; applies to the page fetch, every
   content image, and hero images alike, since asset URLs are
   attacker-influenced page content.
4. **Protocol handler traversal guard** — DB-validated article id plus
   canonicalized path containment, with tests covering forged ids and `..`.
5. **Offline asset rule** — no CDN fonts/icons; everything ships vendored
   (`@fontsource*` packages, inline SVG icons).

## Updates

`tauri-plugin-updater` doesn't cover Android or Linux `.deb`/`.rpm`, so
Legere drives one `latest.json` manifest from GitHub releases in three
platform-specific ways:

- **AppImage** — stock `tauri-plugin-updater` (tauri-action generates the
  manifest entry).
- **`.deb`/`.rpm`** (`commands::update_linux`) — CI patches
  `linux-x86_64-deb`/`linux-x86_64-rpm` entries (asset URL + sha256) into the
  same manifest; the app downloads, verifies the checksum, and installs with
  the system package manager.
- **Android** (`commands::update_android` + `plugins/tauri-plugin-apk-installer`)
  — behind the optional `apk-self-update` Cargo feature (a plain dev build
  has *no* self-update machinery linked in). The app downloads the release
  APK and hands it to the OS package installer via the custom plugin.

All paths compare manifest versions with `semver` and verify sha256
checksums. The repo is public, so all of these GitHub API/release requests
go out unauthenticated.

## Frontend

- **Svelte 5 runes** (`$props`, `$state`, `$effect`, `$derived`) throughout;
  stores are `.svelte.ts` modules.
- **Routes** — `/` (library), `/favorites`, `/category/[id]`,
  `/reader/[id]`, `/sources`, `/settings`. Global dialogs (add source,
  Raindrop import, delete-all, move-to-category) are mounted once in the
  root layout and driven by `uiStore`.
- **Data flow** — commands via `api.ts` (typed `invoke` wrappers; the one
  place the `{kind, message}` error shape from `error.rs` is parsed), events
  via `events.ts` into rune-store refetches.
- **Adapter** — `@sveltejs/adapter-static`; the Tauri webview loads the
  built SPA from disk.
- **Theming** — a single app-wide `light`/`dark` theme (an article may
  override it just for its own reader view); tokens and component rules live
  in [DESIGN.md](DESIGN.md).

## Platform notes

### Linux desktop

`run()` sets `GTK_IM_MODULE=gtk-im-context-simple` for this process only
(before GTK initializes) unless the user already set it: Fedora's default
`ibus-typing-booster` has predictive-completion lookups that freeze the
whole GTK event loop on digit/punctuation input — this is an IBus bug, not
application code, and the workaround stays as cheap hardening.

### Android

- `minSdkVersion` 30 (a firmer modern-WebView floor than Tauri's default 24).
- **TLS init ordering** (`mobile_tls.rs`): `rustls-platform-verifier` needs a
  JNI `Context` handoff before the first TLS handshake. `ndk-context` is
  never populated in a Tauri app (that's the `NativeActivity` model), so
  `MainActivity.kt` calls a JNI-exported `initTls()` from `onCreate`
  instead — after `System.loadLibrary` has run, before any fetch.
- Android's `TextClassifier` is set to `NO_OP` in `MainActivity.kt`: every
  WebView `<input>` is backed by an `EditText` that otherwise runs on-device
  entity detection per edit. (Hardening; not the fix for the input-freeze
  bug, which was garbled titles — see capture/extract above.)
- `gen/android` is committed (see README for the three manual patches and
  why `frontend/` being a sibling of `src-tauri/` requires them).

## Testing approach

Rust tests run fully offline: `test_support.rs` serves `src-tauri/tests/fixtures/`
over an in-process axum server, and tests inject a plain (unguarded)
`reqwest::Client` so localhost fixtures aren't blocked by the SSRF guard.
The end-to-end capture test asserts the important narrowing property —
`content/<id>/` contains only what the readable fragment references, never
page-level assets — plus link cleaning and DB-level dedup. Migration tests
replay real historical schemas (V1 with duplicate/mail rows, etc.) forward.
Sanitizer, SSRF, URL-mapping, protocol-handler, GC, and import tests each
cover their module's invariants, including traversal and adversarial input.
The frontend has `svelte-check` type checking; behavior is exercised through
the Rust-side integration tests and manual QA.

## Historical decisions

A few shapes changed significantly and the reasons are worth keeping:

- **Per-article ZIM archives → plain content directories.** Early designs
  packed the readable view's images into a single-file ZIM served by a
  custom `zim://` protocol (and briefly, full-page server-side archiving
  with a ZIM "Original" viewer — cut for the MVP). Both were removed: plain
  per-article directories under `content/<id>/` served by one protocol
  handler are simpler, need no reader/parsing/LRU machinery, and the schema
  migrations (V3–V6) document the column-by-column retreat.
- **`source_name` dropped (V11).** Capture-time provenance labels ("Direct
  link", "Raindrop import") were superseded by real categories (V9); the
  reader shows the article's live category name instead.
- **Reading theme collapse (V10).** The old separate
  `light/sepia/dark` reader theme became one app-wide `light`/`dark` axis
  with optional per-article overrides; `sepia` is gone, not hidden.
