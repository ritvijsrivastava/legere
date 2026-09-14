# Releasing

Releases are cut by pushing a `v*` tag. `.github/workflows/release.yml` then:

1. **`build-desktop`** — builds `.deb`, `.rpm`, and `.AppImage` bundles on `ubuntu-22.04` and attaches them to a **draft** GitHub release (`Legere vX.Y.Z`). tauri-action's own updater support only covers the AppImage (its `latest.json` gets one `linux-x86_64` entry, for that bundle); a following step patches in `linux-x86_64-deb` and `linux-x86_64-rpm` entries pointing at the just-uploaded `.deb`/`.rpm` assets, with a sha256 checksum each, so the in-app updater can self-update those installs too (see `src-tauri/src/commands/update_linux.rs`).
2. **`build-android`** — builds a signed arm64 `.apk` with `--features apk-self-update` and uploads it as a workflow artifact. Runs concurrently with `build-desktop` (they share no build state), not after it.
3. **`publish-android`** (after both builds succeed) — attaches the APK to the release `build-desktop` created and patches an `android-aarch64` entry into the same `latest.json`.
4. **`publish-release`** (after `publish-android`) — generates release notes from the tag's commit log and flips the draft to published.

The release stays a draft until every artifact has built successfully, so a broken Android build never leaves a public, half-finished release.

## Desktop updater signing

`src-tauri/tauri.conf.json` has `bundle.createUpdaterArtifacts: true` and a `plugins.updater.pubkey`. **The checked-in pubkey is a placeholder** (`REPLACE_ME_RUN_NPM_RUN_TAURI_SIGNER_GENERATE`) — generate a real keypair with:

```sh
npm run tauri signer generate
```

then replace the placeholder with the printed public key. Once *any* pubkey is configured, the bundler hard-errors on **every** `tauri build` (including local, non-release builds) unless `TAURI_SIGNING_PRIVATE_KEY` (and `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`, if the key is password-protected) is set in the environment — this is expected, not a bug, but it means local desktop builds break the moment the real pubkey lands until those env vars are exported locally too. CI reads the same two values from repo secrets.

Keep `appimage` out of `tauri.conf.json`'s `bundle.targets` — it needs FUSE, which most local dev sandboxes lack. It's added at build time via the workflow's `--bundles deb,rpm,appimage` instead; only CI (which has FUSE) builds it.

```sh
npm run release -- patch   # or minor / major — bumps package.json, root Cargo.toml
                            # ([workspace.package].version), src-tauri/tauri.conf.json,
                            # frontend/package.json, and Cargo.lock together in one commit
git ptag                    # tags the committed version and pushes it, refusing to run
                             # if the version wasn't actually bumped since the last tag
```

`npm run release` delegates to `scripts/release.sh`, which runs `npm version <type> --no-git-tag-version` (triggering `scripts/sync-version.mjs`, npm's `version` lifecycle hook, to keep the Rust/Tauri version fields in sync) and commits the result as `chore: bump version to vX.Y.Z`. `git ptag` (a global alias, not part of this repo) then reads the version straight from `package.json`, so the two steps can't drift apart.

(or trigger `release.yml` manually via `workflow_dispatch` for a rebuild without a new tag — `.github/scripts/sync-version.sh` no-ops in that case since `github.ref_name` won't match a `vX.Y.Z` tag.)

A separate `warm-release-cache.yml` workflow rebuilds on every push to `main` (when `src-tauri/**` or `frontend/package-lock.json` change) purely to keep the Rust build caches warm — GitHub Actions caches are scoped per-ref, and a release tag is always new, so without this every release build would be a fully cold `cargo build`.

## Android signing

Play Store requires every update to a given `applicationId` (`com.ritvijsrivastava.legere`) to be signed with the *same* key forever — even though this workflow's APK targets GitHub-release/sideload distribution today, not the Play Store, so getting the signing key right from the first release avoids having to migrate users off an unsigned/differently-signed build later. That signing key:

- is a `.jks` keystore kept **outside the repo**, on the maintainer's machine — never commit it, never regenerate it casually, and back it up somewhere durable.
- is referenced by `src-tauri/gen/android/keystore.properties`, which is **gitignored** — it holds the keystore path, alias, and password locally and is never committed.
- is fed to CI via three repo secrets, base64-encoding the `.jks` file itself:

  | Secret | Contents |
  | --- | --- |
  | `ANDROID_KEY_ALIAS` | the key alias inside the keystore |
  | `ANDROID_KEY_PASSWORD` | the keystore/key password |
  | `ANDROID_KEY_BASE64` | `base64 -w0 upload-keystore.jks` |

  The workflow decodes these into a fresh `keystore.properties` + `.jks` on the runner at build time (see the `Set up Android signing` step) — nothing signing-related is ever persisted in the repo itself.

### `src-tauri/gen/android/app/build.gradle.kts`'s `signingConfigs.release` block

Tauri normally treats `gen/android` as disposable scaffolding you'd regenerate with `tauri android init`. Legere commits it anyway, because `build.gradle.kts` carries a hand-added `signingConfigs.release` block (added following [Tauri's Android signing guide](https://v2.tauri.app/distribute/sign/android/)) that reads `keystore.properties` and applies it to the release build type — without that block, `tauri android build --apk` produces an *unsigned* APK. CI depends on that block existing, so it has to be tracked.

If `gen/android` is ever deleted and recreated via `tauri android init`, this signing config will be silently lost. Recovering it means re-adding the `signingConfigs` block (see the Tauri guide above) and re-pointing a local `keystore.properties` at the **existing** keystore file — do not generate a new one.

## Required secrets checklist

None of these exist yet; the workflows will fail without them:

| Secret | Used for |
| --- | --- |
| `TAURI_SIGNING_PRIVATE_KEY` / `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` | signing desktop updater artifacts (from `npm run tauri signer generate`) |
| `ANDROID_KEY_ALIAS` / `ANDROID_KEY_PASSWORD` / `ANDROID_KEY_BASE64` | signing the release APK |

AppImage bundling requires FUSE, which most local dev sandboxes don't have — only the CI runner builds it locally-equivalent; if you need to test an AppImage build locally, do it on a machine with FUSE available rather than in a sandboxed environment.
