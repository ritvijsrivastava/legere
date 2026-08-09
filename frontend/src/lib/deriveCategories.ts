import type { ArticleSummary } from './types';

/** The sidebar Categories section and the mobile category-chip row both
 *  need the same "distinct source, sorted, with counts" derivation —
 *  kept in one place so they can't drift. */
export function deriveCategories(items: ArticleSummary[]): [string, number][] {
	const counts = new Map<string, number>();
	for (const a of items) {
		counts.set(a.source_name, (counts.get(a.source_name) ?? 0) + 1);
	}
	return [...counts.entries()].sort((a, b) => a[0].localeCompare(b[0]));
}

export function deriveTags(items: ArticleSummary[]): [string, number][] {
	const counts = new Map<string, number>();
	for (const a of items) {
		for (const t of a.tags) counts.set(t, (counts.get(t) ?? 0) + 1);
	}
	return [...counts.entries()].sort((a, b) => a[0].localeCompare(b[0]));
}
