<script lang="ts">
	// Renders a category's chosen glyph — the shared lookup so every place
	// a category shows up (sidebar, article card/row, category chip, search
	// results, the picker itself) resolves an icon id the same way. Fast
	// path: one of the curated pack (`categoryIcons.ts`), rendered
	// synchronously, no async gap. Anything else (one of the ~1800 other
	// Lucide icons picked via `CategoryIconPicker`'s search, or an
	// unrecognized/missing id) falls through to `DynamicIcon`, which lazily
	// loads it and falls back to the generic folder glyph rather than
	// rendering nothing.
	import { findCategoryIcon } from '$lib/categoryIcons';
	import DynamicIcon from './DynamicIcon.svelte';

	let { icon, size = 14 }: { icon: string | null | undefined; size?: number } = $props();
	let curated = $derived(findCategoryIcon(icon));
</script>

<span class="category-icon" style:width="{size}px" style:height="{size}px">
	{#if curated}
		<curated.Icon size={size} />
	{:else}
		<DynamicIcon id={icon || 'folder'} size={size} />
	{/if}
</span>

<style>
	.category-icon {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		flex: none;
		color: var(--color-muted);
	}
</style>
