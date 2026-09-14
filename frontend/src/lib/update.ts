import { invoke, Channel } from '@tauri-apps/api/core';
import { getVersion } from '@tauri-apps/api/app';
import { platform } from '@tauri-apps/plugin-os';
import { relaunch } from '@tauri-apps/plugin-process';

/** Raw update/release metadata returned by the backend. */
export interface UpdateInfo {
	version: string;
	notes: string | null;
	date: string | null;
}

export type UpdateCheckResult = { available: false } | ({ available: true } & UpdateInfo);

export type InstallProgress =
	| { event: 'started'; data: { contentLength: number | null } }
	| { event: 'progress'; data: { chunkLength: number } }
	| { event: 'finished' }
	| { event: 'installing' };

export function isAndroid(): boolean {
	return platform() === 'android';
}

/** Which update mechanism this build ships, mirrored 1:1 from the backend's
 *  `target_os`/Cargo-feature combination (see `update_channel` in
 *  `src-tauri/src/commands/update.rs`) — the single source of truth for
 *  which commands are actually registered, rather than a separately-set Vite
 *  env var that could drift out of sync with what's compiled in. */
export type UpdateChannel = 'desktop' | 'android-github' | 'android-none';

let channelPromise: Promise<UpdateChannel> | null = null;
export function updateChannel(): Promise<UpdateChannel> {
	if (!channelPromise) {
		channelPromise = invoke<UpdateChannel>('update_channel').catch((e) => {
			channelPromise = null;
			throw e;
		});
	}
	return channelPromise;
}

/**
 * How Legere was installed on this Linux machine — `'appimage'`, `'deb'`,
 * `'rpm'`, or `null` if that couldn't be determined (e.g. a dev build, or a
 * binary copied out of a bundle rather than installed through a package
 * manager). Backed by a Rust command that shells out to `dpkg-query`/`rpm`,
 * so it's cached for the life of the session — same reasoning as
 * `releaseNotesCache` below, just single-valued rather than keyed.
 */
let linuxKindPromise: Promise<string | null> | null = null;
export function linuxInstallKind(): Promise<string | null> {
	if (!linuxKindPromise) {
		linuxKindPromise = invoke<string | null>('linux_install_kind').catch((e) => {
			linuxKindPromise = null;
			throw e;
		});
	}
	return linuxKindPromise;
}

/** Currently installed app version, e.g. "0.1.0". */
export function currentVersion(): Promise<string> {
	return getVersion();
}

/** Check the latest release against the installed version. */
export async function checkForUpdate(): Promise<UpdateCheckResult> {
	let command = 'check_for_update';
	if (isAndroid()) {
		if ((await updateChannel()) !== 'android-github') return { available: false };
		command = 'android_check_for_update';
	} else if (platform() === 'linux') {
		const kind = await linuxInstallKind();
		if (kind === 'deb' || kind === 'rpm') command = 'linux_check_for_update';
	}
	const info = await invoke<UpdateInfo | null>(command);
	return info ? { available: true, ...info } : { available: false };
}

// A release's notes never change once published, but the Settings page is
// torn down and rebuilt every time the user navigates away from and back to
// it — without this, its onMount would re-fetch the installed version's
// changelog from GitHub on every single visit. Cached by version for the
// life of the app session; failures are evicted so a transient network
// error can still be retried on the next call.
const releaseNotesCache = new Map<string, Promise<UpdateInfo | null>>();

/**
 * Changelog for a given version's GitHub release. Used both for the
 * currently installed version (Settings' "What's new") and for a pending
 * update's version (the "Details" expansion on the update toast/Settings
 * update row) — deliberately not the `notes` field `checkForUpdate()`
 * returns, since that comes from `latest.json`, which is generated at build
 * time *before* the real changelog is written into the release body (see
 * the backend command's doc comment). Returns `null` if that version has no
 * matching published release (e.g. a local dev build).
 */
export function getReleaseNotes(version: string): Promise<UpdateInfo | null> {
	let cached = releaseNotesCache.get(version);
	if (!cached) {
		cached = invoke<UpdateInfo | null>('get_release_notes', { version }).catch((e) => {
			releaseNotesCache.delete(version);
			throw e;
		});
		releaseNotesCache.set(version, cached);
	}
	return cached;
}

/**
 * The release notes step in `.github/workflows/release.yml` wraps a plain
 * `- subject` commit list in a `<details><summary>Changelog</summary>`
 * block (so it collapses on the GitHub release page). Strip that wrapper and
 * return the commit lines so the app can render its own list instead of
 * dumping the raw markup as text. Also strips the trailing `(hash)` that
 * older, already-published releases still have baked into each line.
 */
export function parseChangelog(notes: string | null | undefined): string[] {
	if (!notes) return [];
	return notes
		.replace(/<\/?details>/g, '')
		.replace(/<summary>.*?<\/summary>/gs, '')
		.split('\n')
		.map((line) => line.trim())
		.filter((line) => line.startsWith('-'))
		.map((line) => line.slice(1).trim().replace(/\s*\([0-9a-f]{7,40}\)\s*$/, ''));
}

/**
 * Download and apply the update found by the preceding `checkForUpdate()`
 * call. On desktop (AppImage/mac/windows) this relaunches the app on
 * success; on Android the OS package installer takes over and this simply
 * resolves once it's launched; on a .deb/.rpm Linux install, the backend
 * elevates via `pkexec` to run `apt`/`dnf` and handles its own relaunch
 * (see `linux_install_update` in `update_linux.rs`) — don't call `relaunch()`
 * for that path, it would race the backend's own respawn.
 */
export async function installUpdate(onProgress?: (p: InstallProgress) => void): Promise<void> {
	const channel = new Channel<InstallProgress>();
	if (onProgress) channel.onmessage = onProgress;

	if (isAndroid()) {
		await invoke('android_download_and_install', { onProgress: channel });
		return;
	}

	if (platform() === 'linux') {
		const kind = await linuxInstallKind();
		if (kind === 'deb' || kind === 'rpm') {
			await invoke('linux_install_update', { onProgress: channel });
			return;
		}
	}

	await invoke('install_update', { onProgress: channel });
	await relaunch();
}

export function getLastDismissedVersion(): Promise<string | null> {
	return invoke<string | null>('get_last_dismissed_version');
}

export async function setLastDismissedVersion(version: string): Promise<void> {
	await invoke('set_last_dismissed_version', { version });
}

/**
 * True only when Legere is running on Linux and couldn't determine how it
 * was installed (not AppImage, not a recognized `.deb`/`.rpm` install) — the
 * one case where auto-update genuinely isn't available and Settings should
 * say so. AppImage/.deb/.rpm all self-update, so none of those trigger this.
 */
export async function shouldShowLinuxUpdateWarning(): Promise<boolean> {
	return platform() === 'linux' && (await linuxInstallKind()) === null;
}
