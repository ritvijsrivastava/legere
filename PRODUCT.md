# Product

<!-- impeccable:product-schema 1 -->

## Platform

web

<!-- Assumption, unconfirmed: Legere ships as installed packages (Tauri
desktop: deb/rpm/AppImage/dmg/nsis/msi, and an Android APK), not a
browser site — but both platforms render the same custom SvelteKit UI
("Nocturne" tokens) with no adoption of per-OS native widgets (no
Material on Android, no AppKit chrome on desktop). Per the design
skill's own rule, a native wrapper around one custom design language is
`web`, not `adaptive`; `adaptive` is reserved for a product that
deliberately speaks a different design language per OS. Flag this if a
genuinely OS-native treatment (e.g. Material You on Android) is ever
wanted instead of one shared identity. -->

## Users

Primary user is the developer/owner reading their own RSS feeds and
saved links — a personal, single-user offline reader (no accounts, no
multi-user backend; independent local SQLite per device, sync
explicitly out of scope). The situation it's built for: reading with an
unreliable or absent connection (commute, flight, spotty wifi) and
wanting a calm, tracking-free, ad-free version of a saved article rather
than the live page. The recurring job: "add this feed/link now, read it
properly later, offline, without the source site's clutter."

## Product Purpose

Legere turns RSS feeds and pasted article links into a small, permanent,
fully offline personal library: it extracts a clean readable version of
each article (Mozilla-Readability-style extraction), localizes that
view's own images so it never needs the network again, and strips
tracking parameters off the original link before storing it. Success is
"I can always reopen anything I've saved and read it, offline, without
noise" — with the reading experience itself, not the plumbing, being
the point of using the app.

## Positioning

Unlike a read-it-later service (Pocket/Instapaper-style) or a full
page-archiver, Legere's mechanism is deliberately narrow: only the
readable extraction and its own images are made durable and offline
(`content/<id>/`); the original page is never archived or rendered
in-app — "view original" is a plain external link, opened in the system
browser, with tracking params already cleaned off it. No server, no
account, no cloud sync: every device keeps its own local library. (An
earlier design captured a full-page ZIM/"Original" viewer per article;
that was cut for the MVP — see PLAN.md's addendum — and is out of scope
unless revisited later.)

## Operating Context

- Tauri 2 (Rust) backend + SvelteKit/Svelte 5 (runes) frontend, SQLite
  storage via r2d2/rusqlite.
- Two shipped targets: Desktop (Linux `.deb`/`.rpm`/AppImage, macOS
  `.dmg`, Windows `.msi`/`.nsis`) and Android (APK, minSdk 30).
- Distributed via signed GitHub releases with an in-app updater
  (desktop self-replace; Android hands off to the OS package installer).
- RSS/Atom polling (autosync toggle) plus direct-URL capture; per-article
  read/reading/read-later state, favorites, and tags (feed-derived at
  capture time, freely editable afterward — always lowercase, no upper
  limit per article).
- No backend service to design for — everything above is local-first.

## Capabilities and Constraints

- Ingestion: RSS/Atom sources and direct article URLs; feed title
  backfill, per-source status (active/paused/error) with retry.
- Reading: readable-view toggle of font size (16–22px), text
  measure/width (narrow/default/wide), line-height (compact/default/
  airy), and theme (light/dark — the same single app-wide theme, not a
  separate reader-only palette). Settings sets the global default for
  all four; the reader's "Aa" popover can override any of them for just
  the article being read (persisted per-article, with a "reset to global
  defaults" action), without touching the global setting. Reading
  progress is persisted and restored (scroll position, the article's
  fixed read time shown in the byline, library-card progress bar).
- Library: card view and list view, filter by user-managed
  folder/category and by tag, unread/favorite counts, delete with confirm,
  "Favorites" as its own view. The Library search box (search-by-title)
  also matches category and tag names, showing matched articles, then
  matched categories, then matched tags, with a scope filter to narrow a
  search to just one of the three; Favorites and single-category pages
  keep the plain by-title search only, since they're already scoped to
  one slice of the library.
- Categories: flat, user-managed folders, managed from the sidebar/library
  (not Settings). Uncategorized is always first, then the rest
  alphabetically. Clicking one opens its own page (title + article list +
  a settings button for rename/delete, delete asks a plain confirm and
  moves its articles back to Uncategorized). Any article can be moved to a
  category from its card/row or the reader's overflow menu, via a dialog
  that lists existing categories as one-click suggestions and can
  create-and-move into a new one in the same step. A fresh direct-link
  capture starts Uncategorized with no prompt.
- **Highlights is a stub today** — the nav item and route exist, the
  reader shows "Tap a marked passage to highlight it and add a note,"
  but no highlighting/annotation feature is implemented yet. Treat it as
  a real near-term surface, not decoration to hide, but don't invent
  interaction details for it beyond what's asked.
- Settings: reading defaults, library default view, autosync on/off,
  source management (not folder/category management — that lives in the
  sidebar/library, see above), in-app update check/install, about.
- Constraint: fully offline-capable by design — no remote fonts/icons,
  vendored fonts only (`@fontsource*`), a real (non-null) CSP, SSRF-
  guarded fetches. Any new visual asset (fonts, icons, the app icon)
  must ship vendored/offline, not loaded from a CDN.
- Constraint: this is a two-platform, one-window-size-to-many
  responsive UI (Tauri desktop window is user-resizable, default
  1100×760, min 360×480) — not a fixed-viewport site. Cards, hero
  images, and controls must hold up correctly across a wide range of
  actual window widths, not just maximized/full-width.

## Brand Commitments

- Name: **Legere** — Latin infinitive, "to read." This is fixed and is
  the one durable naming/etymology fact to design around.
- No other visual identity is locked. The current mark (a plain
  rounded-square app icon with a flat "L" glyph, plus a colored-dot +
  wordmark in the sidebar/topbar) is explicitly called out by the user
  as dated/generic and is an **anti-reference**, not a constraint to
  preserve — it and the current token palette/typography are open for
  full replacement.
- The "Aa" typography popover concept is a working product convention,
  not binding visual style; keep the concept, redesign the execution.
  The reader theme was previously a separate three-way axis (`light` /
  `sepia` / `dark`, independent of the app-wide theme) — collapsed to a
  single app-wide `light`/`dark` theme, overridable per article; `sepia`
  is gone, not a hidden option to resurrect.

## Evidence on Hand

Solo personal project (single GitHub author across the commit history);
no external users, testimonials, customer logos, benchmarks, or
marketing copy exist or should be invented. All real "content" in this
product is the user's own captured articles/feeds — there is no
separate marketing surface to populate with proof.

## Product Principles

1. **Reading is the product.** Chrome, controls, and decoration exist to
   recede once an article is open; nothing should compete with the text
   for attention.
2. **Offline and private by default.** No feature may assume network
   reachability or introduce tracking/remote loading; this extends to
   fonts, icons, and any asset shipped with the app.
3. **One identity, two shells.** Desktop and Android share the same
   visual system and component vocabulary; platform differences are
   handled as layout/density/safe-area adaptations of one design, not
   two designs.
4. **Narrow, honest scope.** Legere reflects exactly what it does
   (readable-view capture, not full-page archiving); the UI should never
   imply capabilities (sync, accounts, full-page snapshots) it doesn't
   have.

## Accessibility & Inclusion

No formal standard has been set by the user. Existing code already
treats a 44px touch target as a floor on mobile controls
(`components.css`'s `.btn-icon` mobile override) and uses
`:focus-visible` outlines — preserve and extend these rather than
introducing a new baseline unasked.
