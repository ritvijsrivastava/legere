---
name: Legere
description: A quiet, offline-first personal reading library — index-card restraint, not a SaaS dashboard.
colors:
  warm-paper: "#faf8f3"
  card-stock: "#f0eae0"
  bright-stock: "#ffffff"
  soft-ink: "#221f19"
  soft-ink-muted: "rgba(34, 31, 25, 0.58)"
  hairline: "rgba(34, 31, 25, 0.12)"
  ledger-pine: "#2d6b50"
  ledger-pine-fg: "#ffffff"
  stamp-red: "#ae3c2e"
  warm-paper-dark: "#1a1814"
  card-stock-dark: "#24211a"
  bright-stock-dark: "#2b271f"
  parchment: "#ece6d8"
  parchment-muted: "rgba(236, 230, 216, 0.6)"
  hairline-dark: "rgba(255, 255, 255, 0.08)"
  ledger-pine-dark: "#7fc39d"
  ledger-pine-dark-fg: "#12231a"
  stamp-red-dark: "#e2887a"
typography:
  display:
    fontFamily: "Literata Variable, Georgia, serif"
    fontSize: "clamp(28px, 5vw, 38px)"
    fontWeight: 600
    lineHeight: 1.15
    letterSpacing: "-0.01em"
  headline:
    fontFamily: "Inter, system-ui, sans-serif"
    fontSize: "30px"
    fontWeight: 700
    lineHeight: 1.15
    letterSpacing: "-0.015em"
  title:
    fontFamily: "Inter, system-ui, sans-serif"
    fontSize: "19px"
    fontWeight: 650
    lineHeight: 1.15
    letterSpacing: "-0.015em"
  body:
    fontFamily: "Inter, system-ui, sans-serif"
    fontSize: "15px"
    fontWeight: 400
    lineHeight: 1.55
  reading:
    fontFamily: "Literata Variable, Georgia, serif"
    fontSize: "19px"
    fontWeight: 400
    lineHeight: 1.75
  label:
    fontFamily: "Inter, system-ui, sans-serif"
    fontSize: "12px"
    fontWeight: 500
    lineHeight: 1.3
    letterSpacing: "0.06em"
rounded:
  sm: "5px"
  md: "9px"
  lg: "15px"
  xl: "20px"
  pill: "999px"
spacing:
  "1": "4px"
  "2": "8px"
  "3": "12px"
  "4": "16px"
  "6": "24px"
  "8": "32px"
components:
  button-primary:
    backgroundColor: "{colors.ledger-pine}"
    textColor: "{colors.ledger-pine-fg}"
    typography: "{typography.title}"
    rounded: "{rounded.md}"
    padding: "8px 14px"
  button-primary-hover:
    backgroundColor: "#296249"
    textColor: "{colors.ledger-pine-fg}"
    rounded: "{rounded.md}"
    padding: "8px 14px"
  button-secondary:
    backgroundColor: "transparent"
    textColor: "{colors.soft-ink}"
    typography: "{typography.title}"
    rounded: "{rounded.md}"
    padding: "8px 14px"
  card:
    backgroundColor: "{colors.card-stock}"
    textColor: "{colors.soft-ink}"
    rounded: "{rounded.lg}"
    padding: "{spacing.3}"
  input:
    backgroundColor: "{colors.card-stock}"
    textColor: "{colors.soft-ink}"
    typography: "{typography.body}"
    rounded: "{rounded.md}"
    padding: "6px 10px"
  tag-accent:
    backgroundColor: "#dcebe3"
    textColor: "{colors.ledger-pine}"
    rounded: "7px"
    padding: "3px 10px"
  dialog:
    backgroundColor: "{colors.bright-stock}"
    textColor: "{colors.soft-ink}"
    rounded: "{rounded.xl}"
    padding: "26px"
---

# Design System: Legere

## Overview

**Creative North Star: "The Field Index"**

Legere reads like a personal index of things worth keeping, not a SaaS dashboard for managing them. The reference points are index cards, specimen labels, and library call-slips: warm paper, a single confident ink accent, quiet hairline structure, and type that gets out of the way except in exactly one place — the moment an article opens and its headline sets in a serif built for reading. Everything in the chrome (nav, buttons, meta, controls) speaks in a plain, workhorse sans; everything in the reading surface speaks in a warm literary serif. The two never trade places.

The system is deliberately narrow-palette: neutrals plus one accent, carried at low-to-moderate coverage (active states, links, the odd filled button) rather than large color fields. Corners are consistently soft but not bubbly; shadows are structural, not decorative, and mostly ride on a hairline ring rather than a heavy drop shadow. Motion is a single snap (one easing curve, two durations) used everywhere small things move — a button press, a card lift, a dialog's entrance — so the whole app feels like one material rather than a pile of separately-tuned transitions.

Confirmed anti-references: the previous default Tauri/Android template icon and a generic dark-navy/lavender "AI SaaS" palette are explicitly rejected — not evidence to preserve, evidence of what this system replaced. A cream-paper-plus-serif "cozy bookish" cliché was also deliberately avoided; the paper tone here is closer to bone/warm-neutral than cream, and the serif is reserved for reading, not chrome.

**Key Characteristics:**
- One accent (Ledger Pine), everywhere else neutral paper and ink.
- Two-font system with a hard boundary: Inter for every control and label, Literata only inside the reading surface.
- Hairline-ring-first elevation; shadow is added, not assumed.
- Pill-shaped segmented controls; soft (not sharp, not maximal) corners on cards and dialogs.
- One motion signature (`ease-snap` + two durations) reused for every interactive transition.

## Colors

Calm and warm at rest, with a single deep, confident green doing all of the system's "this is active / this is mine / this matters" signaling.

### Primary
- **Ledger Pine** (`#2d6b50` light / `#7fc39d` dark): the one accent. Filled buttons, active nav/segment state, links inside prose, favorite markers, focus rings, progress fills. Used for identity too — it's the fill color of the bookmark-ribbon mark. There is deliberately no separate "unread" mark — see the Library section below.

### Neutral
- **Warm Paper** (`#faf8f3` light / `#1a1814` dark): the page background.
- **Card Stock** (`#f0eae0` light / `#24211a` dark): the resting surface for cards, inputs, tag chips — one step off the page.
- **Bright Stock** (`#ffffff` light / `#2b271f` dark): the raised surface for dialogs and popovers — deliberately a step brighter/higher-contrast than Card Stock so an overlay reads as "above" the page.
- **Soft Ink** (`#221f19` light) / **Parchment** (`#ece6d8` dark): primary text.
- **Soft Ink Muted** (`rgba(34,31,25,.58)` light) / **Parchment Muted** (`rgba(236,230,216,.6)` dark): secondary/meta text — always a tint of the ink, never a separate gray.
- **Hairline** (`rgba(34,31,25,.12)` light) / **Hairline Dark** (`rgba(255,255,255,.08)` dark): the one border/divider value used everywhere a rule is needed.

### Named Rules
**The One Accent Rule.** Ledger Pine is the only saturated color in the system. If something new needs emphasis, it gets weight, size, or position — never a second hue.

**The Ink-Tint Rule.** Muted/secondary text is always a transparency of the primary ink color, never an independent gray. This is what keeps light and dark mode feeling like the same palette rather than two different systems.

## Typography

**Chrome Font:** Inter (with system-ui, sans-serif fallback)
**Reading Font:** Literata Variable (with Georgia, serif fallback)

**Character:** A plain, confident workhorse sans runs every control, label, and piece of metadata in the app; a warm literary serif — built specifically for long-form on-screen reading — is reserved for the one surface where reading is the whole point. The pairing is a boundary, not a blend: nothing outside the reader view sets in Literata, and nothing inside `.reader-body`/`.reader-title` sets in Inter.

### Hierarchy
- **Display** (600 variable weight, `clamp(28px, 5vw, 38px)`, 1.15 line-height, Literata): the article title on the reader page. The only large-scale serif moment in the system.
- **Headline** (700, 30px, Inter): page-level `h1` (Library, Settings, Sources...).
- **Title** (650, 19px, Inter): `h3`-level headings, card titles, dialog titles, button label weight.
- **Body — chrome** (400, 15px, 1.55 line-height, Inter): default running text, nav labels, descriptions, all non-reading UI copy.
- **Body — reading** (400, user-tunable 16–22px / default 19px, user-tunable 1.6–1.9 leading / default 1.75, Literata): the article content itself, inside a user-tunable measure (600/680/760px).
- **Label** (500, 12px, 0.06em tracked, Inter, often muted): section labels, `h6`, kicker-style metadata.

### Named Rules
**The Two-Font Boundary Rule.** Inter is chrome, Literata is reading. A component never mixes them, and a new component defaults to Inter unless it renders article prose.

## Layout

A responsive shell: a fixed 256px sidebar (desktop) or a top bar + bottom tab bar (mobile, <768px breakpoint), with the content area scrolling independently. Page content uses a consistent `36px`/`56px` (desktop) or `20px 16px`/`32px` (mobile) padding rhythm.

The library grid holds cards to a **fixed comfortable width** (`repeat(auto-fill, minmax(216px, 264px))` desktop, `minmax(152px, 208px)` on a phone-width grid) rather than letting `1fr` tracks stretch — more columns appear as the window widens; existing columns never balloon. This is a deliberate correction from an earlier version that used `minmax(300px, 1fr)` with a forced 1:1 card aspect ratio, which produced oversized cards in any window narrower than ~3 columns' worth. The mobile tier exists so a phone gets two columns instead of one — `ArticleCollection`'s virtualizer picks the tier from its own measured grid width (not viewport width), so it degrades gracefully on anything in between.

The reader column is capped by a user-selectable measure (narrow 600px / default 680px / wide 760px), centered, independent of window width.

Scrollbars are **thin** (6px, see `tokens.css`) — a classic space-reserving scrollbar is part of the scroll container's content box, so anything wider visibly nudges the content beside it (e.g. the sidebar's rows shifting left) the moment a section grows tall enough to need one. Keep new scrollable areas on this width; don't re-widen or overlay-compensate per component. The desktop sidebar is the one exception: it hides its scrollbar entirely (`scrollbar-width: none` in `Shell.svelte`) — it only overflows when a large section is expanded, and even a thin reserved track shifted its rows left; wheel and keyboard scrolling still work, the handle is just never shown.

Mobile touch targets floor at 44px (`.btn-icon` widens from 36px to 44px under 768px); this floor is load-bearing, not decorative, and should be preserved in any new mobile control.

The mobile collection header (Library/Favorites/category pages) is three rows, not a shrunk desktop toolbar: title + refresh/view-toggle icons share the top row, a full-width search field gets its own row, and category/tag chips scroll in a third. `ArticleCollection`'s `.header-controls` wrapper goes `display: contents` under the mobile breakpoint so its children can become direct CSS Grid participants of `.header-row` without duplicating any markup.

A **bottom sheet** (`MobileTagSheet.svelte`, sharing its list/search logic with the sidebar's Tags section via `TagBrowser.svelte`) is this system's mobile stand-in for a sidebar section that has no room to exist inline — full-width, slides up from the bottom, 20px top corners only, a drag-handle bar, otherwise the same surface/shadow/motion vocabulary as a dialog (`--shadow-lg`, `--ease-snap`).

## Elevation & Depth

Quietly layered, not flat and not heavily lifted. Every surface that needs separation gets a **hairline ring first** (`box-shadow: 0 0 0 1px var(--color-divider)`); a soft ambient shadow is layered on top only for surfaces that sit "above" the page (cards at rest, dialogs) or that respond to interaction (card hover/press). Shadows are warm-tinted (derived from the ink color, e.g. `rgba(38,28,12,...)` in light mode), never neutral black-on-white.

### Shadow Vocabulary
- **`--shadow-sm`** (`0 0 0 1px var(--color-divider)`): hairline-only. Inputs, chips, anything that needs definition but not lift.
- **`--shadow-md`** (`0 0 0 1px var(--color-divider), 0 6px 16px rgba(38,28,12,.1)`): popovers, the Aa panel, the overflow menu.
- **`--shadow-lg`** (`0 0 0 1px var(--color-divider), 0 20px 44px rgba(38,28,12,.16)`): modal dialogs — the deepest lift in the system.
- **`--shadow-card`** (`0 1px 2px rgba(38,28,12,.05), 0 6px 16px rgba(38,28,12,.06)`) → **`--shadow-card-hover`** (`0 3px 8px rgba(38,28,12,.07), 0 14px 28px rgba(38,28,12,.1)`): the one shadow that changes on interaction — a card lifts (`translateY(-3px)`) and its shadow deepens together, eased by `--ease-snap`.

### Named Rules
**The Hairline-First Rule.** Nothing gets a drop shadow before it has a hairline ring; the ring is the resting state, shadow is what interaction or true overlay-elevation adds.

## Shapes

Consistently soft, never sharp and never maximal. Radius scales with the size and formality of the container: `5px` (small controls), `9px` (buttons, inputs, tags), `15px` (cards), `20px` (dialogs), and a full `999px` pill for every segmented control. The bookmark-mark logo uses the same soft-square logic (rounded-corner app icon). No hard/neobrutalist edges, no fully sharp corners anywhere in the system.

## Components

### Buttons
- **Shape:** 9px radius (`--radius-md`); icon buttons are 36px square (44px on mobile).
- **Primary:** Ledger Pine fill, white/near-black text (light/dark), Title-weight (650) label.
- **Secondary:** transparent fill, hairline border, ink text.
- **Ghost:** no border, accent-colored text, used for low-emphasis actions (back button, "Details").
- **Reader actions:** the original article link sits with the reader metadata; sharing is a quiet icon action in the reader header and shares the cleaned external URL rather than a local reader route.
- **Hover/Press:** background darkens (primary) or tints from ink (secondary/ghost) over `--duration-base` with `--ease-snap`; every button scales to 0.96 on press — the one tactile signature shared by every clickable control in the app.

### Segmented Controls
- **Style:** pill-shaped (999px) container, hairline border, options divided by hairline rules.
- **Selected state:** Ledger Pine fill, accent-fg text, weight steps up to 600.
- **Use:** view toggles (cards/list), reader typography (size/width/leading/theme), settings on/off pairs.

### Cards / Containers
- **Corner style:** 15px radius, hero image cropped to the same radius at the top.
- **Background:** Card Stock.
- **Shadow strategy:** `--shadow-card` at rest → `--shadow-card-hover` + 3px lift on hover (see Elevation).
- **Hero image:** owns its own 16:10 aspect ratio (never forces the whole card into a square) and fades in on load.
- **Internal padding:** 14px.

### Inputs / Fields
- **Style:** Card Stock background, hairline border, 9px radius.
- **Focus:** border shifts to Ledger Pine; no glow/ring, the border color change is the whole affordance.

### Navigation
- Sidebar (desktop) / bottom tab bar (mobile): plain-sans labels, active item takes a Card-Stock-tinted pill background and Ledger Pine text/weight. Bottom bar items are full 44px+ touch targets with icon-over-label. The sidebar's Categories section is a collapsible section header (the same uppercase label + rotating-chevron idiom as Tags) listing user-managed flat folders, never provenance labels such as Direct link or Raindrop import. Uncategorized is always first (shown whenever any category is, even at 0 articles); real categories are capped to the busiest 5 by article count so a large library can't push the rest of the sidebar off-screen, with a "Manage categories" row (count on the right, mirroring "Manage tags") linking to `/categories`. Each category links to its own `/category/[id]` page rather than filtering in place.
- The desktop sidebar's top-level items and the mobile bottom bar's tabs are the same four — Library / Favorites / Sources / Settings — as equal top-level destinations ("Add source" and the activity row live above the sidebar's list as separate controls; they're shortcuts, not nav slots).
- A page dedicated to managing a thing shows that thing's actions outright when there's room: `/sources` renders its header actions (Add source, Sync all, Import CSV, Export CSV) and each card's actions (Pause/Make active, Sync now, Remove) as plain buttons on desktop, and only narrow viewports collapse the secondary ones behind ⋮ overflow menus. Don't reach for a ⋮ menu on a wide screen just because mobile needs one.
- **Category identity:** a category shows a small muted-ink glyph, chosen from a fixed pack (`lib/categoryIcons.ts`) via the icon picker on `/categories` or `/category/[id]`'s settings dialog — never a color. Every category defaults to a plain folder glyph until a different one is picked; there's no per-category color coding anywhere in the app.

### Dialogs
- **Style:** 20px radius, Bright Stock background (a step brighter than the page for real overlay separation), `--shadow-lg`.
- **Scrim:** ink-tinted translucent backdrop (`color-mix(in srgb, var(--color-text) 45%, transparent)`) with a light blur — never pure black.
- **Motion:** scrim fades in, dialog pops in with a slight translateY + scale settle, both on `--duration-base`/`--ease-snap`.

### Bottom Sheet (mobile)
- **Use:** the mobile stand-in for a desktop sidebar section that has nowhere to live inline — currently just the Tags browser/search (`MobileTagSheet.svelte`).
- **Style:** full viewport width, anchored to the bottom, 20px radius on the top two corners only, a small centered drag-handle bar, Bright Stock background, `--shadow-lg`, same ink-tinted scrim as a dialog.
- **Motion:** scrim fades in, sheet slides up + fades in, both on `--duration-base`/`--ease-snap` — the same signature as a dialog's entrance, just from the bottom edge instead of a center pop.

### Bookmark Mark (signature component)
The brand mark is a single filled bookmark-ribbon shape (not a lettermark) — a rounded-top rectangle with a V-notch cut at the bottom. It renders in `currentColor` inline next to the "Legere" wordmark (sidebar/topbar) and as a paper-ribbon-on-Ledger-Pine app icon/favicon. It never gets a second color, a gradient, or a drop shadow of its own.

## Do's and Don'ts

### Do:
- **Do** keep Literata inside the reading surface only; every other piece of type is Inter.
- **Do** give every new card/surface a hairline ring before considering a shadow.
- **Do** use the pill (999px) shape for any new segmented/toggle control.
- **Do** reuse `--ease-snap` / `--duration-fast` / `--duration-base` for new motion rather than inventing a new curve.
- **Do** keep muted text as a transparency of the ink color (`color-mix(in srgb, var(--color-text) N%, transparent)`), not a separate gray token.
- **Do** hold grid items to a fixed comfortable width (`minmax(min, cap)`) rather than letting `1fr` stretch them at low column counts.

### Don't:
- **Don't** introduce a second accent hue; intensity comes from weight/size/position, not a new color.
- **Don't** use a hard-edged or zero-blur shadow anywhere — this is not a neobrutalist system.
- **Don't** let a card's aspect ratio be forced square; the hero owns its own ratio, the card's height is intrinsic.
- **Don't** set article/reading prose in Inter, or chrome/controls in Literata.
- **Don't** use pure black scrims or pure gray text — both break the warm-ink-tint identity.
