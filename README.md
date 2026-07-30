# Legere

A local-first, offline-first article reader. Add RSS feeds or paste a
direct link; Legere extracts a readable version, cleans tracking params
off the link, and archives the original page into a self-contained ZIM
file so it stays readable (images included) with no network at all.

Built with Tauri 2 (Rust) + SvelteKit (Svelte 5 runes), SQLite storage,
and the [wraith](../wraith) crates (sanitize/asset-localize/ZIM) as path
dependencies. See `PLAN.md` for the phase-by-phase remodel history.

## Desktop

```sh
npx --prefix frontend tauri dev
```

Run from the **repo root**, not from `frontend/` — the CLI resolves
`tauri.conf.json` (in `src-tauri/`) relative to its own working directory,
and `beforeDevCommand`/`beforeBuildCommand` handle `cd`-ing into
`frontend/` themselves.

## Android

Prerequisites (already expected to be on `PATH`/set for a working Android
toolchain): Android SDK + NDK, a JDK Gradle/AGP can use (JDK 17 or 21 —
**not** a newer default JDK; export `JAVA_HOME` per-command rather than
changing the system default), and [`jq`](https://jqlang.org/) — the
Android build shells out to `cargo metadata | jq` to locate
`rustls-platform-verifier`'s bundled Maven repo (see
`src-tauri/gen/android/app/build.gradle.kts`).

```sh
export JAVA_HOME=/path/to/jdk-21
export ANDROID_HOME=~/Android/Sdk
export NDK_HOME=~/Android/Sdk/ndk/<version>
npx --prefix frontend tauri android build --debug --target x86_64 --apk
```

The APK lands at
`src-tauri/gen/android/app/build/outputs/apk/universal/debug/app-universal-debug.apk`.
Install/launch with `adb install -r <apk>` and
`adb shell am start -n com.ritvijsrivastava.legere/.MainActivity`.

### `gen/android` has manual patches — re-running `tauri android init` reverts them

Legere's `frontend/` is a sibling of `src-tauri/`, not its parent, which
breaks a few of Tauri's default Android-project assumptions. The
generated `gen/android` tree (committed to this repo) has three manual
fixes on top of what `tauri android init` produces:

1. **Root `package.json`** — a minimal delegating package so Gradle's
   internal `npm run tauri` re-invocation can find both a `package.json`
   *and* `src-tauri` as a subfolder from the same working directory (the
   Tauri CLI's own project discovery only searches downward).
2. **`rootDirRel = "../../../../"`** in
   `gen/android/app/build.gradle.kts` (default assumes `frontend/` is
   `src-tauri/`'s parent).
3. **The `rustls-platform-verifier` Maven repo block**, also in
   `app/build.gradle.kts` — `rustls-platform-verifier` needs an actual
   Android `.aar` dependency at runtime (not just the Rust crate), located
   dynamically via `cargo metadata` since its path varies by Cargo
   registry cache layout.

If `gen/android` is ever regenerated from scratch, reapply these three
before building.

## Testing

```sh
cargo test --manifest-path src-tauri/Cargo.toml
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets
npx --prefix frontend svelte-check --tsconfig ./frontend/tsconfig.json
```

Rust tests run fully offline against local fixtures (`src-tauri/tests/fixtures/`,
served by an in-process axum server in `src/test_support.rs` — no network
access, no external services required).
