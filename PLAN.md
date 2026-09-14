# Legere Remodel — Offline Article Reader (Desktop + Android)

## Addendum — offline website archive removed (MVP scope cut)

Everything below describes an earlier plan (and the codebase evolved past
it further still — e.g. full-page archiving was later moved server-side to
a separate `legere-server`, capturing via headless Chromium, polled by an
archive reconciler and cached/evicted locally). That entire feature —
the in-app "Original" archived-page viewer, its `archives/` ZIM cache, the
archive server settings, and all server-capture bookkeeping — has been
**removed** for the MVP. Legere now offers exactly two ways to read an
article: the offline **readable view** (extracted content plus its own
small, persistent `content/<id>.zim` holding just that view's own images —
this part is unchanged and still fully offline-capable), and a plain
**external link to the original site** (`article.link`, opened in the
system browser) — no in-app original-page snapshot. Full-page archiving may
be revisited post-MVP; until then, treat every mention of `archives/`,
`zim_main_path`, `archive_status`/`archive_source`, the archive server, or
an in-app "Original" tab below as historical.

## Addendum — tags (post-MVP)

Tags shipped after the MVP cut below, which still lists them as
deferred (historical). An article can carry any number of
user-and/or-feed-derived tags, always stored lowercase/trimmed/deduped
(`db::queries::normalize_tags`, the single choke point every
tag-writing path — RSS `<category>` capture, Raindrop import, and
manual edits — goes through; migration V8 normalized pre-existing
rows). Sources: RSS `<category>` elements and Raindrop-import tag
cells at capture time, plus manual add/remove in the reader
(`TagEditor.svelte`, via the `set_article_tags` command, replace-all
semantics). Consumed by an any-of tag filter in the library/favorites
views and a sidebar tag list with counts (`list_tags`).

## Addendum — categories/folders (implemented)

Categories are flat, user-managed folders stored in the `categories`
table (migration V9), with nullable `articles.category_id`. Categories
can be empty; deleting one preserves its articles and moves them to
Uncategorized. `articles.source_name` — the old capture-time provenance
label (`Direct link`, `Raindrop import`, or an RSS feed title) that
categories superseded for display purposes — served no further purpose
once nothing rendered it any more and was dropped entirely by migration
V11; the reader now shows an article's live category name in its byline
instead. `sources.name` (the RSS *feed's* own display name, kept in sync
by `queries::set_source_name_if_default`) is unrelated and unaffected.
Category names are unique case-insensitively (`idx_categories_name`);
creating a duplicate name is rejected by the backend, and the one caller
that can hit this on a normal path (`MoveToCategoryDialog`'s "create a
new category") quietly reuses the existing category instead of erroring.

Raindrop CSV imports preview folder names and row counts before capture
(no category choice involved in the preview — it's read-only). A row's
`folder` maps straight to a same-named category automatically when the
import runs (created if it doesn't exist yet, reused case-insensitively
if it does; blank/`Unsorted` means Uncategorized, matching Raindrop's own
convention) via `queries::find_or_create_category`, resolved once per
distinct folder up front rather than per row. A link already saved is
always skipped outright and its category is never changed by a
re-import, even if the folder resolves differently this time; only new
tags the CSV row carries get merged in. Import concurrency (how many
rows capture in parallel) is a user setting, `Settings::import_concurrency`,
clamped to 5–10 on both read and write (Settings section, "Concurrent
captures" stepper) rather than the old fixed constant of 5; it's read
once per import (not live-adjustable mid-run).

Starting an import closes the dialog immediately rather than switching
it to an inline progress view — the import runs in the background
regardless (tracked by the module-level `importStore`, driven by
`import:*` events), and Settings shows a live progress bar plus counts,
with its "Import from Raindrop" button itself turning into
`Importing… N%` while one is running. Once finished the button reverts to
its normal label, but a "Last import: X imported · Y already saved · Z
failed" summary stays visible in Settings (in-memory only, cleared by the
next import or an explicit Dismiss, lost on app restart) so a result
isn't missed just because nobody was watching the dialog when it finished.

Direct-link captures ("Add a source") no longer prompt for a category —
a new article has no `category_id` by construction, so it's simply
Uncategorized until moved. Category management lives outside Settings
entirely: the sidebar lists Uncategorized first (always, whenever any
category is visible, even at 0 articles) followed by real categories
alphabetically (`LibraryStatsStore`), each linking to a dedicated
`/category/[id]` page (`ArticleCollection` scoped via
`libraryFiltersStore.categoryId`) whose header carries a settings button
(hidden for the virtual Uncategorized id) opening `CategorySettingsDialog`
for rename/delete. Deleting asks a plain `confirm()` (articles fall back
to Uncategorized via `ON DELETE SET NULL`, same as before) rather than a
typed confirmation — unlike "delete all articles", this is low-stakes and
non-destructive to content. Moving an article to a category is available
from its card/row (hover action) and the reader's overflow menu, all
opening the one globally-mounted `MoveToCategoryDialog` (`uiStore`),
which lists existing categories as one-click suggestions alongside a
"name a new or existing category" field that creates-and-moves in one
step.

## Addendum — input freeze on `*`/digits, root-caused to a garbled title

Typing `*` (or digits) into the library search box could freeze the whole
window for several seconds, with even unrelated keystrokes like Backspace
not registering until it cleared. Two platform-level theories were tried
first and kept as real, worthwhile hardening even though **neither was
the actual cause**: Android's `TextClassifier` (every WebView `<input>`
is backed by a real `EditText`, which by default runs on-device
phone/address entity detection on each edit — disabled in
`gen/android/.../MainActivity.kt` via `webView.textClassifier =
TextClassifier.NO_OP`), and a suspected IBus/`ibus-typing-booster`
predictive-completion slowdown on Linux desktop (worked around in
`run()`, `src-tauri/src/lib.rs`, by setting
`GTK_IM_MODULE=gtk-im-context-simple` before Tauri/GTK initialize, unless
the user already set that themselves). Both were ruled out by direct
evidence — a `sqlite3` timing check against the real database showed the
search query itself was never slow, and no `ibus` daemon was even running
in the live session — so the freeze had to be reproduced live: the app
was relaunched under `perf record`/`strace -f` attached to the real
process while reproducing the input by hand.

Before that trace was even needed, the user found it by hand: one
article's title was binary garbage, and removing that article made the
lag disappear entirely. The actual mechanism: `capture::fetch::fetch_page`
reads a response body via `reqwest::Response::text()`, which does a
*lossy* charset-aware decode that never fails — a misconfigured or
actually-binary response (wrong `Content-Type`, a compression/charset
mismatch, etc.) still decodes "successfully" into a `String` that's valid
UTF-8 by Rust's own type guarantee, just full of `\u{FFFD}` replacement
characters. If that garbage happened to land in whatever `extract::extract`
pulled out as the title, it got stored as the article's title verbatim —
and a title that's mostly high-entropy replacement/control-character
noise is exactly the shape of input (like dense combining marks or an
unbreakable multi-KB "word") known to make text shaping/line-breaking
degrade badly in browser engines, hanging the whole renderer once it's
displayed *or matched against a search query* (both card/list titles and
the search box's own value get compared against it on every keystroke).

Fixed at the single choke point both extraction paths
(`from_article`/`naive_fallback`) go through in
`src-tauri/src/capture/extract.rs`: `sanitize_title` rejects a title
outright (falls back to "Untitled") if it contains *any* `\u{FFFD}` —
proof the source was never proper UTF-8 to begin with — folds stray
control characters into spaces as a second line of defense, and caps the
result to 300 chars (`MAX_TITLE_CHARS`) so no title, garbled or not, can
ever again be pathologically large. The two platform-level mitigations
above stay in place as unrelated, still-legitimate hardening; the search
input's `spellcheck="false" autocomplete="off" autocorrect="off"
autocapitalize="off"` attributes likewise stay as cheap, harmless-
everywhere defaults, without having been the fix either.

## Context

Legere (`~/Code/legere`, Tauri 2 + SvelteKit/Svelte 5) is an existing MVP scaffold (~1k lines Rust, ~2.1k frontend, 2 commits) for an offline article reader. The user wants RSS/Atom + direct-URL ingestion where each article stores three things: (1) extracted readable HTML, (2) a self-contained single-page **.zim** archive (reusing the sibling `~/Code/wraith` crates, consumed as path deps), and (3) the original link with tracking params stripped. Reading experience anchored on **Matter**. Exploration found the scaffold sound but with structural gaps: the ZIM is write-only and not self-contained (raw HTML, zero assets), extracted articles reference remote images (offline reading is broken), `wraith-urlx` is imported but never called (and wraith has **no** tracking-param stripping anywhere — new work), there is no event system (library never refreshes after autosync), errored sources are unrecoverable, and archive/media files leak forever.

**Verdict: remodel, not rewrite.** Worth keeping: Nocturne token CSS (`frontend/src/lib/styles/`), the single capture pipeline shared by RSS + direct ingestion, the schema skeleton, rune stores, routes, r2d2/rusqlite plumbing. Worth replacing: the ZIM/archive layer, the refresh model, dead mail/direct branches, the naive extraction fallback.

**User decisions (locked):**
- Independent local SQLite stores per device; no sync (schema kept sync-friendly: UUID PKs, add `updated_at`).
- In-app ZIM viewer in v1 (reader toggles Readable ↔ Original archived page).
- wraith stays as **path deps** for now (git-pin TODO in `src-tauri/Cargo.toml` stays deferred).
- Dark-only Nocturne for v1; Matter treatment applied to reader typography.
- MVP scope: read/unread ✅; no tags, no search, no OPML, no notifications, no mail (remove stubs). (Tags shipped post-MVP — see the addendum above.)
- Android minSdk: 30 (Tauri default is 24; 30 chosen as a firmer modern-WebView floor).

**First execution step:** write this plan as `PLAN.md` in `~/Code/legere` (the user asked for the plan as a markdown file in the repo).

---

## Cross-cutting design decisions

- **D1 — ZIM is the asset store.** Readable-view images are served *out of the article's ZIM* via a custom `zim://` protocol; no duplicate asset files. Exception: hero *thumbnails* stay in `media/<uuid>.jpg` (resized JPEGs for library cards); their leak is fixed by delete-time GC, not removal.
- **D2 — `zim://` protocol.** `register_uri_scheme_protocol("zim", …)` in `src-tauri/src/lib.rs`. Frontend builds URLs with `convertFileSrc('<article_id>/<local_path>', 'zim')` → `zim://localhost/…` on Linux, `http://zim.localhost/…` on Android. Handler parses both shapes.
- **D3 — Neutral token in stored HTML.** Capture rewrites readable-HTML asset URLs to `legere-zim:/<article_id>/<local_path>`. Reader does one `replaceAll('legere-zim:/', convertFileSrc('', 'zim'))` before `{@html}`. DB stays platform-neutral.
- **D4 — wraith changes (user owns both repos):**
  1. `wraith-urlx`: new `strip_tracking_params(url: &Url) -> Url` — curated blocklist (`utm_*` prefix; exact: `fbclid gclid gclsrc dclid msclkid wbraid gbraid mc_cid mc_eid igshid igsh si _hsenc _hsmi mkt_tok s_kwcid vero_id oly_anon_id oly_enc_id ck_subscriber_id`; do **not** strip bare `ref` — too many false positives). `canonicalize` semantics untouched (archive paths depend on byte-preservation).
  2. `wraith-assets`: expose the already-computed `url_map: HashMap<Url, LocalPath>` on `LocalizedPage` (verified: built as a local at `crates/assets/src/lib.rs:107`; mechanical change).
  3. `wraith-zim`: add `get_entry_by_full_path → Option<(Vec<u8>, String)>` (bytes + mimetype) to `reader.rs` — protocol handler needs `Content-Type`.
- **D5 — Events.** Backend emits `sync:started`, `sync:finished {new_article_count, errors}`, `articles:changed`, `source:changed` via `AppHandle::emit`. Frontend registers `listen()` in `+layout.svelte`, dispatching coarse-grained store `refresh()`es (dataset is small; refetch beats payload sync).
- **D6 — Structured errors.** New `src-tauri/src/error.rs`: `AppError` enum (thiserror + Serialize → `{kind, message}`); commands return `Result<T, AppError>`. Introduced on touched paths in Phase 1, full sweep in Phase 5.

---

## Phase 1 — Capture & storage correctness

Goal: every new capture has offline-correct readable HTML, a genuinely self-contained ZIM, and a cleaned link.

**wraith first:** D4.1 (`crates/urlx/src/clean.rs` + re-export + unit tests) and D4.2.

**Pipeline rework, `src-tauri/src/capture/mod.rs::capture_article`:**
1. `fetch_page` (unchanged, SSRF-guarded client).
2. Readability-extract from the raw page (`dom_smoothie`, unchanged).
3. Sanitize the **raw page** (`wraith_sanitize::sanitize`) — security boundary for the archive view.
4. New `capture/localize.rs`: `wraith_assets::localize_html(sanitized, final_url, client, ~6, &HashMap::new(), FetchPolicy, &AssetCache)` → `LocalizedPage { html, assets, url_map }`. Cap total asset bytes (skip oversized, leave remote — degrades gracefully).
5. Rewrite `capture/archive.rs`: main page at `ensure_html_extension(local_path_for(canonicalize(final_url)))`; `add_content` for page + every `LocalizedAsset` with its mime; Zstd. Internal hrefs are already relative (wraith `relative_path`), so it renders from `zim://` as-is.
6. New `capture/rewrite.rs`: on the sanitized *extracted* fragment, rewrite `img[src|srcset]`, `source[srcset]`, `video[poster]` → resolve vs `final_url` → `canonicalize` → look up `url_map` → `legere-zim:/<id>/<path>` on hit, leave remote on miss. Use a real HTML rewriter (check if dom_smoothie's parser is reusable before adding `lol_html`) — no regex.
7. Link: `strip_tracking_params(canonicalize(final_url))` stored as display/dedup link.
8. Hero: reuse bytes from `url_map`/fetched assets when available; else existing fetch+resize.
9. Fix `naive_fallback` (`capture/extract.rs`): never store the raw page as `content_html`; store sanitized body text and set new `extraction_confident: bool` (reader defaults such articles to the archive view).
10. `CaptureOutput` gains `zim_main_path: String`.

**Schema V2** (`src-tauri/src/db/schema.rs`, table-recreate since SQLite can't alter CHECKs):
- `sources.type` CHECK → `('rss')`; add `updated_at`.
- `articles.source_type` CHECK → `('rss','direct')`; add `updated_at`, `reading_progress REAL DEFAULT 0`, `zim_main_path TEXT DEFAULT 'index.html'`, `extraction_confident INT DEFAULT 1`; `CREATE UNIQUE INDEX ON articles(link)`.
- Dedup: UNIQUE on cleaned link + `INSERT OR IGNORE`, check `changes()`; drop racy `article_link_exists` pre-check in `sources/rss.rs`. V2 copy uses `INSERT OR IGNORE … ORDER BY fetched_at DESC` (dup collapse acceptable — personal 2-commit-old data).
- All mutating queries set `updated_at`.
- Settings backfill: `default_font_size` small/medium/large → numeric `reader_font_size`.

**Dead-branch removal (backend):** delete the `"mail"` arm in `commands/sources.rs`; trim `models.rs`.

**Verification:** replace both live-network tests with an in-test localhost fixture server (`axum` or `wiremock` dev-dep; fixture page + CSS + images + srcset under `src-tauri/tests/fixtures/`; plain `reqwest::Client` injection bypasses the SSRF guard for 127.0.0.1). Assert ZIM contains page+assets with mimes, `content_html` has `legere-zim:/` tokens and zero remote fixture URLs, link cleaned, duplicate insert is a no-op. Unit tests: `strip_tracking_params`, fallback, rewrite pass. Migration test: raw-V1 temp DB → migrate → assert. **Note: Phases 1+2 ship together** — Phase 1 alone leaves `legere-zim:/` tokens unrenderable.

## Phase 2 — ZIM serving, archive viewer, Matter-grade reader

**Protocol server**, new `src-tauri/src/zim_server.rs`:
- `ZimCache`: `Mutex<LruCache<article_id, Arc<ZimReader>>>` cap 4 (`lru` dep) — `ZimReader::open` loads whole file into RAM; per-article ZIMs are a few MB, on-demand + LRU is fine.
- `serve(state, article_id, path)`: validate UUID exists in DB (blocks traversal into arbitrary `archives/*`), `get_entry_by_full_path("C/{path}")` (D4.3), 404 on miss. `Cache-Control: public, max-age=31536000, immutable` (write-once).
- Register in `lib.rs`; evict on article delete (Phase 3 hook).

**CSP** — replace `csp: null` in `tauri.conf.json` with a real policy: `default-src 'self'`; `script-src 'self'`; `style-src 'self' 'unsafe-inline'` (Svelte + archived inline styles); `img-src`/`media-src`/`frame-src` allow `asset:`/`zim:` and their `http://*.localhost` Android forms; `connect-src ipc: http://ipc.localhost`. Scripts stay locked — sanitize-at-capture is layer 1, CSP layer 2. *Risk: first-time CSP on WebKitGTK — budget an iterate-with-devtools loop.*

**Archive viewer** (`frontend/src/routes/reader/[id]/+page.svelte`): `Readable | Original` toggle (reuse existing `SegmentedControl.svelte`, currently imported-but-unused); Original renders `<iframe sandbox="" src={convertFileSrc(`${id}/${zim_main_path}`, 'zim')}>` full-width. Default = Readable unless `!extraction_confident`. `ArticleDetail`/`types.ts` gain `zim_main_path`, `extraction_confident`, `reading_progress`. Readable view applies the D3 substitution.

**Matter typography:** vendor **Source Serif 4** variable via `@fontsource-variable/source-serif-4` (Inter stays for chrome); tokens `--font-reading`, `--reading-size` (default 19px), `--reading-measure` (600/680/760px), `--reading-leading` (1.6/1.75/1.9); serif title ~clamp(28px,5vw,38px); blockquote/figure/code pass in a scoped `reader.css`. Replace S/M/L with an "Aa" popover (size stepper 16–22, measure, leading) persisted to settings KV. **Persist reading progress:** debounced (750ms) `save_reading_progress(id, f64)` command from the existing scroll handler; restore scroll on open; "X min left" in meta; thin progress bar on library cards. Header fades on scroll-down.

**Verification:** `zim_server` unit tests against an in-test `ZimWriter` fixture (page, asset, 404, `../` traversal, bad id). Manual via `npx --prefix frontend tauri dev` **from repo root** (CLI can't find `tauri.conf.json` from `frontend/`): capture image-heavy article → kill network → readable images render, Original renders styled, no CSP violations, scroll restores after restart. Run `npm run build` in `frontend/` — verifies the inline-adapter/vite config actually builds; extract a conventional `svelte.config.js` if it doesn't.

## Phase 3 — Sources/library UX, events, article lifecycle

- **Events (D5):** new `src-tauri/src/events.rs`; thread `AppHandle` into `sync.rs::sync_all_sources`; emit from sync + source/article commands. Frontend `lib/events.ts` + `uiStore.syncing` spinner + minimal toast for sync errors. Fixes never-refreshing library.
- **Sync UX / error recovery:** Refresh button on library (`syncAll` exists, currently unreachable); per-source "Sync now" + error rows show `last_error` with Retry (`sync_source` already works for errored sources; `mark_source_synced` already resets `status='active'` + clears `last_error` — verified in `db/queries.rs:202-206`). Autosync loop (`sync.rs:33`) includes `status == 'error'` sources (retry each cycle, no backoff for MVP); fix pause-guard `AND status != 'error'` (`queries.rs:190-191`).
- **Feed titles:** `sources/rss.rs` returns parsed `feed.title`; `UPDATE sources SET name=? WHERE id=? AND name=feed_url` (only replace the placeholder, never a user rename).
- **Delete + GC:** `delete_article(id)` command — row, `archives/<id>.zim`, `media/<id>.jpg`, ZimCache eviction, `articles:changed`. Startup orphan sweep in `setup` (spawned): delete `archives/`+`media/` files whose UUID stem has no row. `remove_source` keeps SET NULL (read-later semantics), dialog copy says so. Delete affordance on cards + reader overflow with confirm.
- **Delete all articles (settings "Danger zone"):** `delete_all_articles` command — bulk `DELETE FROM articles` + reset every `sources.article_count` to 0, wholesale `remove_dir_all` on `content/`+`media/` (cheaper than per-row cleanup when everything's going away), `articles:changed`. Sources themselves are kept. Frontend gates the action behind `DeleteAllArticlesDialog.svelte`, a type-`DELETE`-to-confirm modal (not a plain OK/Cancel) given the action is irreversible.
- **Re-capture action** per article (reuses `capture_article` with existing id) — fixes pre-remodel V1 articles with broken content; no bulk migration.
- **Mail stub removal (frontend):** `AddSourceDialog.svelte` → two modes, "RSS feed" and "Article URL"; delete `icons/Mail.svelte`; purge `mail` from `types.ts`.
- **Verification:** delete-GC + orphan-sweep tests (temp dirs), feed-title test with local fixture XML, error→retry state test. Manual: bad feed URL → error + retry; autosync insert appears without restart; deleted article's files gone.

## Phase 4 — Android port

- **Scaffolding:** `rustup target add aarch64-linux-android x86_64-linux-android`; SDK/NDK + env; `npx --prefix frontend tauri android init` → `src-tauri/gen/android`; `bundle.android.minSdkVersion: 30`.
- **TLS — highest-uncertainty item in the plan; spike first** (single HTTPS fetch on emulator before anything else):
  - `aws-lc-sys` (via wraith-assets' reqwest→rustls) officially supports aarch64-linux-android but is a heavy C/asm build. Escape hatch if it fights: switch reqwest features to `rustls-no-provider` in `wraith/crates/assets/Cargo.toml` + legere's reqwest, add `ring`, install `rustls::crypto::ring::default_provider()` at the top of `run()`.
  - `rustls-platform-verifier` needs JNI/Android-context init **before the first TLS handshake**. Tauri's glue populates `ndk-context`; add the verifier init in the `#[cfg(mobile)]` path of setup and verify with a debug log. Fallback: small Kotlin hook in `gen/android` MainActivity.
  - `zstd-sys` (wraith-zim): expected clean with NDK clang.
- **Runtime:** switch `.run(ctx)` to `.build(ctx)…run(closure)` and handle mobile `RunEvent::Resumed` → throttled sync-on-foreground (>5 min since last, timestamp in `AppState`). Background autosync death is accepted for MVP (WorkManager deferred). Safe-area insets on `Shell.svelte` bottom tabs + reader header; touch targets ≥44px; verify hero images resolve via asset protocol on Android paths; `zim` handler already parses `http://zim.localhost` (Phase 2).
- **Verification (x86_64 AVD, API 30+):** boot; add feed over real network (TLS proof); capture completes; readable images via `http://zim.localhost`; Original iframe; airplane-mode reading; relaunch → scroll resume + foreground sync; settings persist. Then one arm64 physical-device build.

## Phase 5 — Polish & hardening

- Finish `AppError` rollout (settings/system commands); `api.ts` narrows `{kind, message}` → toast copy per kind.
- Consolidate fixture server into `src-tauri/tests/common/`; both old live-network tests gone.
- Verify ZIM LRU memory ceiling with a ~20MB archive; PRAGMA review (`db/pool.rs` — keep busy_timeout-first ordering, known startup-race fix).
- Housekeeping: unused-icon audit, remove `as unknown as string` bind cast in `AddSourceDialog`, README, keep the git-pin TODO (explicitly deferred).
- Explicitly deferred: search, OPML, notifications, WorkManager sync, cross-device sync, light theme (tokens kept theme-ready). (Tags shipped post-MVP — see the addendum near the top of this file.)

---

## Top risks (ranked)

1. **Android TLS/JNI init ordering** (Phase 4) — timeboxed spike before the rest of the port.
2. **CSP introduction breaking views on WebKitGTK** (Phase 2) — iterate with devtools.
3. **Asset-localization blowups on heavy pages** (Phase 1) — byte cap, oversized assets left remote.
4. **V2 table-recreate migration collapsing dup rows** — accepted, personal data, documented in the migration comment.

## Critical files

- `src-tauri/src/capture/{mod,archive,extract}.rs` + new `localize.rs`, `rewrite.rs` — pipeline rework
- `src-tauri/src/db/schema.rs`, `db/queries.rs` — V2 migration + dedup + updated_at
- `src-tauri/src/lib.rs` — zim protocol, events, mobile entry/TLS init, RunEvent::Resumed
- new `src-tauri/src/zim_server.rs`, `events.rs`, `error.rs`
- `frontend/src/routes/reader/[id]/+page.svelte`, `lib/styles/tokens.css` + new `reader.css` — viewer toggle + Matter typography
- `wraith/crates/urlx/src/` (new `clean.rs`), `wraith/crates/assets/src/lib.rs` (expose `url_map`), `wraith/crates/zim/src/reader.rs` (mime getter)
