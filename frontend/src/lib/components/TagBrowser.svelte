<script lang="ts">
	// Shared tag search/browse/select logic — used by the desktop sidebar's
	// collapsible "Tags" section (`Shell.svelte`) and the mobile `Tags` sheet
	// (`MobileTagSheet.svelte`). Owns its own search box, facet-narrowed
	// list, and pinned-selected-tags row; the two callers differ only in
	// chrome (collapsible header vs. sheet header) and how many rows they
	// have room to show at once.
	import Search from '$lib/icons/Search.svelte';
	import X from '$lib/icons/X.svelte';
	import { libraryFiltersStore } from '$lib/stores/libraryFilters.svelte';
	import { libraryStatsStore } from '$lib/stores/libraryStats.svelte';
	import * as api from '$lib/api';
	import type { NamedCount } from '$lib/types';

	let {
		maxVisible = 7,
		onSelect
	}: {
		/** Caps the browsable (non-pinned) list — the sidebar has limited
		 *  vertical room; the mobile sheet has a whole screen. */
		maxVisible?: number;
		/** Called after a tag is toggled — the mobile sheet uses this to
		 *  close itself once a selection is made. */
		onSelect?: (tag: string) => void;
	} = $props();

	let tags = $derived(libraryStatsStore.tags);
	let tagSearch = $state('');
	let facetTags = $state<NamedCount[]>([]);

	// Re-scopes the browsable list to only tags that actually co-occur
	// with the current selection (computed server-side by
	// `list_tags_filtered`, using the same filters as the article list
	// itself) — matches the sidebar's original behavior exactly.
	$effect(() => {
		const categoryId = libraryFiltersStore.categoryId;
		const selected = libraryFiltersStore.tags;
		void libraryStatsStore.changeVersion;

		if (!categoryId && selected.length === 0) {
			facetTags = libraryStatsStore.tags;
			return;
		}

		let cancelled = false;
		api
			.listTagsFiltered({ category_id: categoryId, tags: selected })
			.then((result) => {
				if (!cancelled) {
					// Busiest tags first when the list is narrowed (inside a
					// category, or with tag filters active) — mirrors the
					// Categories list's count-driven ordering. Ties keep the
					// backend's alphabetical order (Array.sort is stable).
					facetTags = result.slice().sort((a, b) => b[1] - a[1]);
				}
			})
			.catch(() => {
				// Best-effort UI narrowing — leave the previous list showing.
			});
		return () => {
			cancelled = true;
		};
	});

	let visibleTags = $derived(
		facetTags.filter(
			([tag]) =>
				!libraryFiltersStore.tags.includes(tag) && tag.includes(tagSearch.trim().toLowerCase())
		)
	);

	function select(tag: string) {
		libraryFiltersStore.toggleTag(tag);
		onSelect?.(tag);
	}
</script>

<div class="tag-browser">
	<div class="tag-search">
		<Search size={13} />
		<input
			type="text"
			placeholder="Search tags..."
			bind:value={tagSearch}
			spellcheck="false"
			autocomplete="off"
			autocorrect="off"
			autocapitalize="off"
		/>
	</div>
	{#if libraryFiltersStore.tags.length > 0}
		<div class="tag-list tag-list-selected">
			{#each libraryFiltersStore.tags as tag (tag)}
				<div class="tag-row tag-row-selected">
					<span class="row-label">#{tag}</span>
					<button
						class="tag-remove"
						aria-label={`Remove ${tag} filter`}
						onclick={() => select(tag)}
					>
						<X size={11} />
					</button>
				</div>
			{/each}
		</div>
	{/if}
	<div class="tag-list">
		{#each visibleTags.slice(0, maxVisible) as [tag, count] (tag)}
			<button class="tag-row" onclick={() => select(tag)}>
				<span class="row-label">#{tag}</span>
				<span class="nav-count">{count}</span>
			</button>
		{/each}
		{#if visibleTags.length === 0}
			<div class="tag-empty">No tags match</div>
		{/if}
	</div>
	<a href="/tags" class="manage-tags-link">
		<span>Manage tags</span>
		<span class="nav-count">{tags.length}</span>
	</a>
</div>

<style>
	.tag-browser {
		display: flex;
		flex-direction: column;
	}
	.tag-search {
		display: flex;
		align-items: center;
		gap: 7px;
		background: var(--color-surface);
		border-radius: 10px;
		padding: 7px 10px;
		margin: 0 0 10px;
		color: var(--color-muted);
	}
	.tag-search input {
		border: none;
		background: transparent;
		outline: none;
		font: inherit;
		font-size: 12.5px;
		width: 100%;
		color: var(--color-text);
	}
	.tag-list {
		display: flex;
		flex-direction: column;
		gap: 1px;
	}
	.tag-list-selected {
		margin-bottom: 4px;
	}
	.tag-row {
		display: flex;
		align-items: center;
		gap: 8px;
		width: 100%;
		background: transparent;
		border: none;
		border-radius: 8px;
		padding: 6px 10px;
		cursor: pointer;
		font-family: var(--font-body);
		font-size: 12.5px;
		color: var(--color-text);
		text-align: left;
	}
	.tag-row:hover {
		background: var(--color-surface);
	}
	.tag-row-selected {
		background: color-mix(in srgb, var(--color-accent) 14%, var(--color-surface));
		color: var(--color-accent);
		cursor: default;
		font-weight: 600;
	}
	.tag-row .row-label {
		flex: 1;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.nav-count {
		font-size: 11px;
		font-family: var(--font-body);
		color: var(--color-muted);
	}
	.tag-remove {
		display: flex;
		align-items: center;
		justify-content: center;
		background: none;
		border: none;
		color: inherit;
		cursor: pointer;
		padding: 3px;
		border-radius: 6px;
		flex: none;
	}
	.tag-remove:hover {
		background: color-mix(in srgb, var(--color-accent) 20%, transparent);
	}
	.tag-empty {
		padding: 8px 10px;
		font-size: 12px;
		color: var(--color-muted);
	}
	.manage-tags-link {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 8px;
		margin: 10px 2px 0;
		padding: 6px 10px;
		border-radius: 8px;
		font-size: 12.5px;
		font-weight: 500;
		color: var(--color-text);
		text-decoration: none;
	}
	.manage-tags-link:hover {
		background: var(--color-surface);
		color: var(--color-accent);
	}

	/* Touch-comfortable rows on mobile (the sheet is only ever shown
	   there) — taller tap targets than the desktop sidebar's compact
	   rows, still within the same visual language. */
	@media (max-width: 768px) {
		.tag-search {
			padding: 10px 12px;
		}
		.tag-search input {
			font-size: 14px;
		}
		.tag-row {
			padding: 11px 10px;
			font-size: 14px;
			border-radius: 10px;
		}
		.tag-remove {
			padding: 6px;
		}
		.manage-tags-link {
			padding: 11px 10px;
			font-size: 13.5px;
		}
	}
</style>
