import { openUrl } from '@tauri-apps/plugin-opener';
import { isTauri } from './platform';

export type ShareResult = 'shared' | 'copied' | 'cancelled';

/** Opens a URL in the device's default browser. The web fallback keeps the
 *  same behavior when the frontend is run without the Tauri shell. */
export async function openExternalUrl(url: string): Promise<void> {
	if (isTauri()) {
		await openUrl(url);
		return;
	}

	const opened = window.open(url, '_blank', 'noopener,noreferrer');
	if (!opened) {
		throw new Error('The browser blocked the original article link.');
	}
}

/** Shares the original article URL where the platform supports it, falling
 *  back to the clipboard for desktop browsers and WebViews without a native
 *  share sheet. */
export async function shareArticleLink(title: string, url: string): Promise<ShareResult> {
	if (typeof navigator === 'undefined') {
		throw new Error('Sharing is not available here.');
	}

	if (typeof navigator.share === 'function') {
		try {
			await navigator.share({ title, url });
			return 'shared';
		} catch (error) {
			if (isShareCancellation(error)) return 'cancelled';
			// A share sheet can exist but still reject on a platform that has no
			// configured share target. Continue to the clipboard fallback.
		}
	}

	if (navigator.clipboard && typeof navigator.clipboard.writeText === 'function') {
		await navigator.clipboard.writeText(url);
		return 'copied';
	}

	throw new Error('Sharing is not available on this device.');
}

function isShareCancellation(error: unknown): boolean {
	return (
		(error instanceof DOMException && error.name === 'AbortError') ||
		(typeof error === 'object' &&
			error !== null &&
			'name' in error &&
			error.name === 'AbortError')
	);
}
