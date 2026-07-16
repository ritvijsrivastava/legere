export function formatDate(iso: string | null): string {
	if (!iso) return '';
	const date = new Date(iso);
	if (Number.isNaN(date.getTime())) return '';
	return date.toLocaleDateString(undefined, { month: 'short', day: 'numeric' });
}

export function formatReadTime(minutes: number): string {
	return `${minutes} min`;
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
