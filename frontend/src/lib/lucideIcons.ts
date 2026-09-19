// The full Lucide set (1848 icons, generated from `@lucide/svelte` by
// `scripts/generate-lucide-icons.mjs` into `lib/icons/lucide/*.svelte` —
// one flat inline-SVG component per icon, same shape/stroke style as every
// other hand-vendored icon in `lib/icons/`). Vendored as plain files rather
// than imported from the npm package at runtime so this is (a) offline-safe
// the same way the rest of the app's assets are and (b) lazily code-split
// per icon via `import.meta.glob`'s non-eager form, so picking from 1848
// options doesn't mean bundling 1848 components into the main chunk —
// only the ones a user actually searches for/selects ever load.
import type { Component } from 'svelte';

const modules = import.meta.glob('./icons/lucide/*.svelte') as Record<
	string,
	() => Promise<{ default: Component<{ size?: number }> }>
>;

/** Every vendored icon's id (its Lucide kebab-case name, e.g. `'cat'`,
 *  `'alarm-clock'`), sorted — the searchable universe for
 *  `CategoryIconPicker`. */
export const LUCIDE_ICON_IDS: string[] = Object.keys(modules)
	.map((path) => path.slice('./icons/lucide/'.length, -'.svelte'.length))
	.sort();

const ID_SET = new Set(LUCIDE_ICON_IDS);

export function isLucideIconId(id: string): boolean {
	return ID_SET.has(id);
}

/** Kebab-case id \u2192 human label for search matching and a11y labels,
 *  e.g. `'alarm-clock'` \u2192 `'Alarm Clock'`. Derived rather than stored
 *  since storing a hand-written label for 1848 entries isn't worth it. */
export function lucideIconLabel(id: string): string {
	return id.replace(/-/g, ' ').replace(/\b\w/g, (c) => c.toUpperCase());
}

/** Lazily loads one icon's component. Called per-icon as picker results
 *  render (or as a category's chosen icon needs display), never upfront. */
export function loadLucideIcon(id: string): Promise<Component<{ size?: number }>> {
	const load = modules[`./icons/lucide/${id}.svelte`];
	if (!load) return Promise.reject(new Error(`Unknown lucide icon: ${id}`));
	return load().then((mod) => mod.default);
}
