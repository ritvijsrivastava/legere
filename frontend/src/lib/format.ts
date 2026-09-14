export function formatDate(iso: string | null): string {
	if (!iso) return '';
	const date = new Date(iso);
	if (Number.isNaN(date.getTime())) return '';
	return date.toLocaleDateString(undefined, { month: 'short', day: 'numeric' });
}

export function formatReadTime(minutes: number): string {
	return `${minutes} min`;
}

/** Just the host of an article's URL (e.g. `example.com` for
 *  `https://example.com/v1/feed.html`), stripping a leading `www.` since
 *  it adds noise without adding information. Falls back to the raw link
 *  if it isn't a parseable absolute URL. */
export function formatHost(link: string): string {
	try {
		return new URL(link).hostname.replace(/^www\./, '');
	} catch {
		return link;
	}
}

/** Same relative-time buckets as `formatRelativeTime`, abbreviated
 *  (`2d ago` vs `2 days ago`) for tight spaces like the reader header. */
export function formatCompactRelativeTime(iso: string | null): string {
	if (!iso) return '';
	const date = new Date(iso);
	if (Number.isNaN(date.getTime())) return '';

	const diffMs = Date.now() - date.getTime();
	const diffMin = Math.max(0, Math.round(diffMs / 60_000));
	if (diffMin < 1) return 'now';
	if (diffMin < 60) return `${diffMin}m ago`;
	const diffHours = Math.round(diffMin / 60);
	if (diffHours < 24) return `${diffHours}h ago`;
	const diffDays = Math.round(diffHours / 24);
	if (diffDays < 7) return `${diffDays}d ago`;
	return date.toLocaleDateString(undefined, { month: 'short', day: 'numeric' });
}

export function formatRelativeTime(iso: string | null): string {
	if (!iso) return 'never';
	const date = new Date(iso);
	if (Number.isNaN(date.getTime())) return 'never';

	const diffMs = Date.now() - date.getTime();
	const diffMin = Math.round(diffMs / 60_000);
	if (diffMin < 1) return 'just now';
	if (diffMin < 60) return `${diffMin} min ago`;
	const diffHours = Math.round(diffMin / 60);
	if (diffHours < 24) return `${diffHours} hour${diffHours === 1 ? '' : 's'} ago`;
	const diffDays = Math.round(diffHours / 24);
	return `${diffDays} day${diffDays === 1 ? '' : 's'} ago`;
}
