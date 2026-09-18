# Plan — "Original page" snapshot per article

Status: proposal, not started. Decisions already taken with the user are
marked **[decided]**; open ones are collected at the end.

## 0. Verdict on obscura

`h4ckf0r0day/obscura` is a Rust headless browser that embeds V8
(`obscura-js` → `deno_core 0.350`). It is **not usable as an in-app
dependency** for Legere:

| Constraint | Obscura |
|---|---|
| App size | ~70 MiB release binary (its own README); V8 alone is tens of MB. Legere's whole point is a small offline reader. |
| Android | Release matrix is Linux x86_64/arm64, macOS, Windows only. Its V8 startup snapshot is baked per-architecture at build time; no mobile target exists. |
| Packaging | Not on crates.io (the `obscura` crate there is an unrelated raytracer). Only consumable as a git dependency on a 9-crate workspace; the `stealth` feature needs CMake/Clang/LLVM. |
| What it buys | Executing page JS. This repo already ran a real engine at capture time once (`d904ffa`, WebKitGTK + Android WebView) and deliberately turned JS **off** because a real site's analytics beacons fired on every capture. |
| Output | `--dump html/text/markdown/assets` — no single-file/MHTML archive. Asset localization would still be Legere's own work. |

What obscura would do (fetch → run JS → hand back DOM) is a ~50-line
shell-out on desktop if a user installs it themselves. That is a
possible future backend for §7 Tier B, but it is dominated by using the
platform's own WebView (already shipped, zero size, works on Android), so
it is **not** in this plan.

**Feasible instead, at ~zero binary cost:** a static full-page snapshot.
The raw page HTML is already in memory in `capture_local` (`page.html`),
the URL→path mapping, `content/<id>/` store, `legere-content://` server,
orphan GC and delete paths all exist, the CSP already allows
`frame-src legere-content:`, and the old "Original" viewer was exactly an
`<iframe sandbox="">` on that protocol. The CSS tokenizer needed
(`cssparser`) is already compiled into the binary via
`dom_smoothie → dom_query → selectors`. New code is the full-page
localize/rewrite pass, a schema column, two commands, and a reader toggle.

## 1. Scope **[decided]**

Each article can hold three things:

1. **Readable view** — unchanged.
2. **Original page** — a static, sanitized, fully localized snapshot of
   the page as the server sent it: HTML + stylesheets + fonts + images,
   scripts stripped, rendered in-app inside a sandboxed iframe, offline.
3. **Link** — unchanged.

Policy **[decided]**: a global setting `snapshot_on_capture`, **default
OFF**. When on, every capture path (RSS sync, direct link, Raindrop
import, re-capture) also snapshots. Regardless of the setting, the reader
offers per-article **Save original page** / **Remove original page**.
Scripts are **stripped** (no inert copy, no `raw.html`). Viewer is a
**Reader | Original** toggle in the reader header.

Explicitly out of scope: running JS inside the app, archiving iframes
recursively, video/audio bodies over the per-asset cap, service workers,
any change to the readable pipeline's output.

## 2. Storage layout

```
content/<id>/
  original.html                    ← the snapshot document (new)
  https/<host>/<path…>             ← assets, same mapping as today
```

- `original.html` can never collide with an asset path: every asset path
  starts with a scheme segment (`http/`, `https/`).
- Snapshot and readable view **share one directory and one URL→path
  mapping**, so an image referenced by both is fetched and stored once.
- All references inside `original.html` and inside localized CSS are
  **relative paths** (`https/host/a.css`, `../fonts.host/f.woff2`), not
  `legere-content:/` tokens. The iframe loads the file straight from the
  protocol handler with no frontend token pass, and relative paths are
  platform-neutral (desktop `legere-content://localhost/<id>/…`, Android
  `http://legere-content.localhost/<id>/…`) with no runtime rewriting.
  `<base>` is stripped so relatives resolve against the document itself.
- Schema **V13**: `ALTER TABLE articles ADD COLUMN snapshot_path TEXT;`
  NULL = no snapshot; otherwise `original.html` (relative to the content
  dir, mirroring `hero_image_path`'s relative-path convention). No backfill.
- Settings KV: new key `snapshot_on_capture` (`"true"`/`"false"`),
  default `false`, surfaced on `Settings` and `models::Settings`.

## 3. Backend

### 3.1 Pipeline shape (`capture/mod.rs`)

`capture_local(client, data_dir, id, url, opts: CaptureOptions { snapshot: bool })`.
One page fetch feeds both views:

```
fetch ─┬─ extract → sanitize(fragment) → discover readable refs ─┐
       └─ sanitize_page(full doc) → discover page refs ──────────┤
                                                                  ▼
                       localize(union of refs, with CSS recursion)
                                                                  ▼
       rewrite readable fragment (tokens)  +  rewrite page (relative)
                                                                  ▼
       archive: wipe dir, write assets, write original.html
```

`LocalCaptureOutput` gains `snapshot_path: Option<String>`. A snapshot
failure **never** fails the capture: log at `warn`, store `None`, the
readable view is saved exactly as today. Insert/update queries write the
new column.

Snapshot-only entry point for the on-demand action:
`capture_snapshot_only(client, data_dir, id, url)` — fetch, build the
snapshot, write assets **additively** (no dir wipe — the readable view's
images are already there) and `original.html`, return the path. Re-capture
(existing action) passes `snapshot: existing.snapshot_path.is_some() || setting`
so a per-article choice survives a re-capture.

Call sites to thread the option through: `sources/direct_link.rs:26`,
`sources/rss.rs:66`, `sources/raindrop_import.rs:184`,
`commands/articles.rs:353`. Each reads the setting once per run (RSS: once
per sync, import: once per import) rather than per article.

### 3.2 Full-page sanitizer (`capture/sanitize.rs`)

Keep `sanitize()` (fragment) byte-for-byte as is — its tests protect the
readable view. Add `sanitize_page()` that applies the same denylist plus,
for a whole document:

- unwrap `<noscript>` (keep children) — lazy-load fallbacks become the
  real content once JS is gone;
- promote lazy-loading attributes: `data-src`→`src`, `data-srcset`→`srcset`,
  `data-lazy-src`, `data-original` (only when the target attribute is
  missing or is a 1×1 placeholder/`data:` URI);
- remove `link[rel~=preload|prefetch|modulepreload|preconnect|dns-prefetch|manifest|serviceworker]`;
- remove **every** `meta[http-equiv=refresh]` (offline redirect is never useful);
- remove `object`, `embed`, `applet`; `iframe` → existing placeholder;
- strip `integrity` and `crossorigin` attributes (localized CSS is
  rewritten, so SRI hashes would fail);
- neutralize `form[action]` (drop the attribute);
- remove existing `meta[charset]`/`meta[http-equiv=content-type]` and
  prepend `<meta charset="utf-8">` to `<head>` (stored bytes are UTF-8);
- prepend a document CSP `<meta http-equiv="Content-Security-Policy">`
  (see §5).

### 3.3 Reference discovery + CSS (`capture/localize.rs`, new `capture/css.rs`)

Discovery over the full document, on top of today's `img[src|srcset]`,
`source[srcset]`, `video[poster]`:

- `link[rel~=stylesheet][href]`, `link[rel~=icon][href]`;
- `source[src]`, `video[src]`, `audio[src]` (subject to the media cap);
- `svg image[href|xlink:href]`;
- `style` attributes and `<style>` elements → `url()` / `@import` via
  `css.rs`;
- every fetched `.css` asset → parsed recursively for `@import` and
  `url()` (fonts, backgrounds, masks, cursors, `image-set()`), **depth ≤ 3**.

`css.rs` uses `cssparser` (add as a direct dep pinned to the version
already in `Cargo.lock`; no new code compiled) to tokenize, collecting the
byte spans of `Url`/`UnquotedUrl` tokens and `@import` preludes, then
rewrites those spans to relative paths. Skips `data:`, `blob:`, `#frag`
and non-http(s). Strips a UTF-8 BOM and any `@charset` rule before
parsing (a BOM once silently broke a static CSS parse in this repo's
history — `render.rs` doc in `d904ffa`). Comments and escapes are handled
by the tokenizer, not by regex.

Relative path computation between two `LocalPath`s (`a/b/c.css` →
`x/y.woff2` = `../../x/y.woff2`) is a small pure function with tests.

### 3.4 Budgets (snapshot only; readable view keeps its 20 MB rule)

| Cap | Value | On overflow |
|---|---|---|
| per asset | 8 MB | left remote |
| total per snapshot | 40 MB | remaining refs left remote |
| asset count | 250 | remaining refs left remote |
| CSS import depth | 3 | deeper imports left remote |
| concurrency | 6 (existing) | — |

Every asset fetch goes through the existing SSRF-guarded client and
`literal_ip_is_blocked` pre-check, unchanged.

### 3.5 Commands (`commands/articles.rs`, `lib.rs` handler list)

- `save_original_page(id) -> ArticleDetail` — on-demand; runs
  `capture_snapshot_only`, sets `snapshot_path`, emits `articles:changed`.
- `remove_original_page(id) -> ArticleDetail` — deletes `original.html`,
  then every file under `content/<id>/` **not** referenced by the readable
  view (derive the keep-set by scanning `content_html` for
  `legere-content:/<id>/<path>` tokens — deterministic, no manifest),
  clears `snapshot_path`.
- `get_settings`/`update_settings` gain `snapshot_on_capture`.

`delete_article`, `delete_all_articles`, and `gc::sweep_orphaned_files`
need **no change** — they already operate on the whole `content/<id>/` dir.

### 3.6 Serving (`content_server.rs`)

- `guess_content_type` gains `html`, `css`, `woff`, `woff2`, `ttf`, `otf`,
  `eot`, `json`, `xml` (svg already present).
- Everything is still served `immutable`; the frontend cache-busts the
  iframe URL with `?v=<updated_at>` so a re-saved snapshot isn't served
  stale from the webview cache.
- Add a `Content-Security-Policy` response header on `.html` entries
  identical to the injected meta (belt and braces; header wins where
  supported).

## 4. Frontend

- `types.ts`/`models.rs`: `ArticleDetail` gains `snapshot_path: string | null`
  and `updated_at: string` (cache-bust). `Settings` gains
  `snapshot_on_capture: boolean`.
- `api.ts`: `saveOriginalPage(id)`, `removeOriginalPage(id)`.
- **Reader** (`routes/reader/[id]/+page.svelte`):
  - a `.seg` segmented control **Reader | Original** in `.header-row`
    next to `ReaderControls`, rendered only when `snapshot_path` is set;
  - Original mode replaces `.reader-page` with a full-bleed
    `<iframe class="original-frame" sandbox="" title={article.title}
    src={convertFileSrc(`${id}/original.html`, 'legere-content') + '?v=' + updated_at}>`
    sized to the remaining viewport height; the progress bar, scroll
    tracking/saving and the per-article theme override apply to Reader
    mode only;
  - mode is per-visit (not persisted) and defaults to Reader; when
    `!extraction_confident` and a snapshot exists, default to Original
    (the old design's rule — the readable view is known-poor there).
- **Overflow menu** (`ArticleOverflowMenu.svelte`): add
  `Save original page` (spinner while running; toast on failure) or
  `Remove original page` depending on state, above `Re-capture`.
- **Settings**: a toggle "Also save the original page — uses more storage
  (typically 1–8 MB per article) and slows feed sync", same two-radio
  pattern as autosync. Import dialog gets a one-line note when the
  setting is on.
- Library cards: no change (a snapshot indicator isn't worth the clutter).

## 5. Security model (adds a third layer, changes nothing existing)

1. **Sanitize at capture** — `sanitize_page` strips every execution
   vector, same denylist plus the additions in §3.2.
2. **Sandbox at render** — `<iframe sandbox="">` with no `allow-scripts`,
   no `allow-same-origin`, no `allow-top-navigation`, no `allow-forms`,
   no `allow-popups`. The snapshot is an opaque origin that can't script,
   navigate the app, or reach IPC.
3. **Document CSP** on the snapshot itself:
   `default-src 'none'; script-src 'none'; object-src 'none'; frame-src 'none';
   base-uri 'none'; form-action 'none'; style-src 'self' 'unsafe-inline';
   img-src 'self' data: https: http:; font-src 'self' data:; media-src 'self'`.
   `'self'` resolves to the `legere-content` origin on both platforms.
   Remote `https:` images are allowed so a reference that failed to
   localize degrades the same way the readable view does (shows online,
   blank offline).
4. The app's own CSP (`tauri.conf.json`) already limits `frame-src` to
   `legere-content:`; clicking a link inside the snapshot therefore does
   nothing (navigation blocked). See open question Q2.
5. SSRF: unchanged guard on every fetch; the traversal guard in
   `content_server` covers `original.html` like any entry.

## 6. Tests

- **Fixtures**: extend `tests/fixtures/style.css` with an `@import
  "theme.css"`, a `@font-face { src: url(fonts/a.woff2) }`, and a
  `background: url("bg.png")`; add `theme.css`, `fonts/a.woff2` (any
  bytes), `bg.png`; add a `<script>`, a `<noscript><img src=lazy.jpg>`,
  a `<link rel=preload>`, an `integrity=` attribute and a `data-src`
  image to `article.html`. The Quiet Harbor text is untouched so existing
  extraction assertions hold.
- `capture/mod.rs`: the existing end-to-end test becomes the
  `snapshot: false` case (**asserts `style.css` absent, unchanged**); a
  new `snapshot: true` case asserts `original.html` present, `style.css`,
  `theme.css`, the font and `bg.png` present, no `<script>`/`integrity`/
  `preload` in the document, `lazy.jpg` promoted, every localized
  reference relative (no fixture-server URL), readable `content_html`
  unchanged vs the `false` case.
- `capture/css.rs`: unit tests for quoted/unquoted/escaped `url()`,
  `@import` both forms, `data:` skipped, BOM/`@charset` stripped,
  relative-path computation, depth cap.
- `sanitize_page`: adversarial-input test (mirror of the fragment one),
  noscript unwrap, lazy promotion, preload/refresh/integrity removal.
- `content_server`: MIME for `.html`/`.css`/`.woff2`; CSP header present
  on `.html`.
- `queries`: V13 migration replay; `remove_original_page` keep-set logic
  (readable image kept, snapshot-only CSS removed).
- Manual QA (desktop + Android emulator): image/font-heavy article →
  Original renders styled offline; sandbox blocks a link click; re-save
  shows fresh content (cache-bust); Reader progress unaffected by Original
  mode; setting off ⇒ no `original.html` after an RSS sync.

## 7. Tier B — JS-rendered pages (optional follow-up, keeps Android)

The static snapshot faithfully captures server-rendered pages, which is
most article sites. Client-rendered pages (SPAs, some paywalls, infinite
lazy-load) come out as an empty shell. The user's concern here is real
but only affects a minority of pages, so the answer is an **on-demand,
per-article "Re-save with JavaScript"** action — never automatic — using
the engine the app already ships:

- **Desktop (one code path for Linux/macOS/Windows)**: a hidden
  `WebviewWindowBuilder::new(app, "snapshot-capture", WebviewUrl::External(url))`
  with `.visible(false)`, `.incognito(true)` (no persisted cookies),
  `.on_navigation(...)` rejecting non-http(s) and blocked literal IPs, and
  an `initialization_script` (main frame only) carrying a per-capture
  nonce. After `PageLoadEvent::Finished` plus a 1.5 s quiet period (hard
  cap 20 s) it serializes `document.documentElement.outerHTML` and submits
  it through one command, `submit_rendered_dom(nonce, html)`, allowed
  for remote origins **only on that window label** via a capability
  `remote.urls` entry. The window is destroyed immediately after. The
  returned HTML then goes through the exact §3 static pipeline (sanitize,
  guarded asset fetches, rewrite, store) — the webview only ever loads the
  page, never the assets.
- **Android**: Tauri's mobile multi-window creates a visible Activity,
  so revive `plugins/page-capture-plugin` from `d904ffa` (Kotlin, ~130
  lines) with `javaScriptEnabled = true`, drop its resource-relaying
  `shouldInterceptRequest`, and return `outerHTML` via
  `evaluateJavascript`. No remote IPC needed there.
- Detection hint: after a static capture, if `!extraction_confident`, or
  the body's visible text is < 400 chars while the page had ≥ 5 script
  tags, show "This page may need JavaScript — Re-save with JavaScript" in
  the Original view. Computed at capture time, not stored.
- Privacy/SSRF trade-offs, stated in the UI: the page's scripts run once,
  like opening it in a browser; the page load itself uses the OS network
  stack (redirect targets are checked in `on_navigation`; DNS-rebinding on
  that single load is accepted for a user-initiated action).
- Size cost: zero on desktop; the Android plugin is a few KB.

This tier is where obscura *could* alternatively plug in on desktop as an
external binary, but the WebView route needs no extra install and also
covers Android, so it wins.

## 8. Phases

| Phase | Scope | Size |
|---|---|---|
| 1 | Schema V13, `CaptureOptions`, `sanitize_page`, `css.rs`, full-page discovery/localize/rewrite, budgets, archive writes, fixtures + tests | L |
| 2 | Commands (save/remove/settings), `content_server` MIME + CSP, GC keep-set, tests | M |
| 3 | Reader toggle + iframe, overflow actions, Settings toggle, import note, manual QA desktop + Android | M |
| 4 | Docs: ARCHITECTURE (principle 4, pipeline, storage, security), PRODUCT (positioning, principle 4, capabilities), README features — required by AGENTS.md, done in the same change as the code | S |
| 5 (optional) | Tier B desktop hidden-window renderer + Android plugin revival + hint | L |

Phases 1–4 ship together (a snapshot with no viewer, or a viewer with no
snapshots, is not a useful intermediate). Estimated binary growth for
1–4: well under 0.5 MB (new Rust code only; every crate is already
linked). Storage growth: 1–8 MB per snapshotted article, hard-capped at
40 MB.

## 9. Risks

1. `'self'` in the snapshot's CSP inside a `sandbox=""` (opaque-origin)
   frame — verify on WebKitGTK and Android WebView first; fallback is
   listing `legere-content:` and `http://legere-content.localhost`
   explicitly.
2. `immutable` caching serving a stale `original.html` after re-save —
   covered by the `?v=` cache-bust; verify WebKitGTK honors it.
3. RSS sync time when the setting is on (30 entries × a few MB): default
   OFF, and snapshot failures/timeouts never block the article.
4. Sites whose CSS is behind a CDN that blocks non-browser user agents —
   degrades to unstyled; note in the Original view when zero stylesheets
   localized.
5. Tier B remote-IPC capability: scoped to one window label and one
   nonce-checked command; on Linux/Android Tauri can't tell an iframe from
   the top frame, so the nonce (main-frame-only init script) is the guard.
   Needs its own security review before merge.

## 10. Open questions

- **Q1 — Tier B now or later?** Plan Phase 5 in this change, or ship
  1–4 first and revisit after seeing how many saved pages actually need
  JS? (Recommendation: ship 1–4 first; the detection hint can land with
  Phase 5.)
- **Q2 — Links inside the Original view.** Currently they'd do nothing
  (sandbox + app CSP). Alternatives: rewrite every `<a href>` to an
  absolute URL and open it in the system browser via a click-through
  overlay (needs no scripts in the frame — the parent can't see the
  click, so this would be a "links open externally" banner with the page
  URL only), or leave as-is and document. (Recommendation: leave as-is
  for v1.)
- **Q3 — Media bodies.** Localize `<video>`/`<audio>` sources up to the
  8 MB cap, or always leave them remote? (Recommendation: leave remote —
  posters are captured, playback is not an offline-reading feature.)
- **Q4 — Default view when the readable extraction was low-confidence and
  a snapshot exists**: Original (old design) or still Reader?
  (Recommendation: Original.)
