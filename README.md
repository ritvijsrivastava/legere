# Legere

A local-first, offline-first article reader for desktop and Android. Add an
RSS feed or paste an article link; Legere extracts a clean, readable version
of the article, localizes the images that readable view references into a
per-article directory on disk, and stores the original link with tracking
parameters stripped. Everything you've saved stays fully readable with no
network at all — the original page itself is never archived; "view original"
opens the cleaned live link in your system browser (or shares it via the
platform share sheet).

Built with Tauri 2 (Rust) + SvelteKit (Svelte 5 runes) + SQLite. No accounts,
no server, no sync — every device keeps its own independent local library.

## Features

- **Feeds and direct links** — RSS/Atom sources with a 15-minute autosync
  loop, or one-shot direct-link capture; the "Add a source" dialog sniffs
  which one a pasted URL is automatically.
- **True offline reading** — readability extraction (`dom_smoothie`, a Rust
  port of Mozilla's Readability) plus localization of the readable view's own
  images into `content/<id>/`, served into the webview through a custom
  `legere-content://` protocol.
- **Cleaned links** — canonicalized URLs with `utm_*`/`fbclid`/`gclid`-style
  tracking parameters stripped before anything is stored.
- **Raindrop.io import** — preview a CSV export's folders before capture;
  each folder auto-maps to a same-named category, already-saved links are
  skipped, and imports run in a cancellable background worker pool.
- **Library** — card/list views, favorites, unread/reading/read state,
  reading-progress restore, user-managed flat categories, tags (from RSS
  categories, Raindrop, or manual edits), and a search box that matches
  article titles, category names, and tag names.
- **Reading experience** — serif reading typography with per-article
  overrides of font size, text measure, line height, and theme on top of
  global defaults.
- **Self-updating** — desktop builds (AppImage/.deb/.rpm) and Android
  sideload builds check GitHub releases for updates in-app.

## Documentation

- [ARCHITECTURE.md](ARCHITECTURE.md) — how the app works: capture pipeline,
  storage layout, security model, and the decisions behind them.
- [PRODUCT.md](PRODUCT.md) — product positioning, scope, and principles.
- [DESIGN.md](DESIGN.md) — the design system (colors, typography, components).
- [docs/RELEASING.md](docs/RELEASING.md) — release process, signing, and CI.

## Development

Prerequisites: Rust (stable), Node.js 22, and the Tauri 2 system dependencies
for your platform (e.g. `libwebkit2gtk-4.1-dev` on Linux — see
[Tauri's prerequisites](https://v2.tauri.app/start/prerequisites/)).

**Run from the repo root, not `frontend/`** — Tauri CLI commands resolve
`tauri.conf.json` (in `src-tauri/`) relative to the working directory, and
`beforeDevCommand`/`beforeBuildCommand` handle `cd`-ing into `frontend/`
themselves.

```sh
npm install --prefix frontend   # frontend deps
npx --prefix frontend tauri dev # desktop dev build
```

### Android

Prerequisites on `PATH`/in the environment: Android SDK + NDK, a JDK Gradle
can use (JDK 17 or 21 — **not** a newer default JDK; export `JAVA_HOME`
per-command rather than changing the system default), and
[`jq`](https://jqlang.org/) — the Android build shells out to
`cargo metadata | jq` to locate `rustls-platform-verifier`'s bundled Maven
repo (see `src-tauri/gen/android/app/build.gradle.kts`).

```sh
export JAVA_HOME=/path/to/jdk-21
export ANDROID_HOME=~/Android/Sdk
export NDK_HOME=~/Android/Sdk/ndk/<version>
npx --prefix frontend tauri android build --debug --target x86_64 --apk
```

For frontend live reload (one-time Rust build, then Vite HMR into the
app on the device), use `dev` instead — the same env vars apply:

```sh
npx --prefix frontend tauri android dev --target x86_64
```

This relies on `TAURI_DEV_HOST` (exported by the Tauri CLI) being honored
in `frontend/vite.config.ts`, so the dev server binds to a network
interface the device can reach.

The APK lands at
`src-tauri/gen/android/app/build/outputs/apk/universal/debug/app-universal-debug.apk`.
Install/launch with `adb install -r <apk>` and
`adb shell am start -n com.ritvijsrivastava.legere/.MainActivity`.

#### `gen/android` carries manual patches — re-running `tauri android init` reverts them

Legere's `frontend/` is a sibling of `src-tauri/`, not its parent, which
breaks a few of Tauri's default Android-project assumptions. The generated
`gen/android` tree (committed to this repo) has three manual fixes on top of
what `tauri android init` produces:

1. **Root `package.json`** — a minimal delegating package so Gradle's
   internal `npm run tauri` re-invocation can find both a `package.json`
   *and* `src-tauri` as a subfolder from the same working directory (the
   Tauri CLI's own project discovery only searches downward).
2. **`rootDirRel = "../../../../"`** in
   `gen/android/app/build.gradle.kts` (the default assumes `frontend/` is
   `src-tauri/`'s parent).
3. **The `rustls-platform-verifier` Maven repo block**, also in
   `app/build.gradle.kts` — `rustls-platform-verifier` needs an actual
   Android `.aar` dependency at runtime (not just the Rust crate), located
   dynamically via `cargo metadata` since its path varies by Cargo registry
   cache layout.

If `gen/android` is ever regenerated from scratch, reapply these three before
building. (A release build additionally needs the `signingConfigs.release`
block described in [docs/RELEASING.md](docs/RELEASING.md).)

### Testing

```sh
cargo test --manifest-path src-tauri/Cargo.toml
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets
npx --prefix frontend svelte-check --tsconfig ./frontend/tsconfig.json
```

Rust tests run fully offline against local fixtures (`src-tauri/tests/fixtures/`,
served by an in-process axum server in `src/test_support.rs` — no network
access, no external services required).

## Status

Legere is a personal project, developed in the open. It works today on Linux
desktop and Android; macOS/Windows targets are configured but less exercised.
There is no sync, no accounts, and no telemetry — by design.

## License

[MIT](LICENSE)
