import { platform } from '@tauri-apps/plugin-os';

export function isTauri(): boolean {
	return typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;
}

/** Android vs. desktop — gates mobile-only UI (e.g. the device-link QR
 *  scan flow in cross-device sync setup) the same way `update.ts` already
 *  gates its own Android-only update mechanism. */
export function isAndroid(): boolean {
	return platform() === 'android';
}
