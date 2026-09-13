<script lang="ts">
	import { settingsStore } from '$lib/stores/settings.svelte';
	import { sourcesStore } from '$lib/stores/sources.svelte';
	import { uiStore } from '$lib/stores/ui.svelte';
	import { articlesStore } from '$lib/stores/articles.svelte';
	import { libraryFiltersStore } from '$lib/stores/libraryFilters.svelte';
	import { deriveCategories } from '$lib/deriveCategories';
	import ArticleCard from './ArticleCard.svelte';
	import ArticleListRow from './ArticleListRow.svelte';
	import Search from '$lib/icons/Search.svelte';
	import Grid from '$lib/icons/Grid.svelte';
	import ListIcon from '$lib/icons/ListIcon.svelte';
	import Refresh from '$lib/icons/Refresh.svelte';
	import type { ArticleSummary, LibraryView } from '$lib/types';

	let {
		title,
		subtitle,
		items,
		emptyMessage,
		showRefresh = false,
		onopen,
		ondelete
	}: {
		title: string;
		subtitle?: string;
		items: ArticleSummary[];
		emptyMessage: string;
		showRefresh?: boolean;
		onopen: (id: string) => void;
		ondelete: (article: ArticleSummary) => void;
	} = $props();

	let search = $state('');
	// The `filtered` derived below re-scans the whole (unbounded, until it's
	// paginated) items array on every dependency change — debounce so fast
	// typing doesn't force a recompute + full re-render per keystroke.
	let debouncedSearch = $state('');
	let searchTimer: ReturnType<typeof setTimeout> | undefined;
	$effect(() => {
		const value = search;
		clearTimeout(searchTimer);
		searchTimer = setTimeout(() => {
			debouncedSearch = value;
		}, 120);
		return () => clearTimeout(searchTimer);
	});

	let libraryView = $state<LibraryView>('cards');
	let initializedFromSettings = false;

	$effect(() => {
		if (settingsStore.loaded && !initializedFromSettings) {
			libraryView = settingsStore.current.default_library_view;
			initializedFromSettings = true;
		}
	});

	function setView(view: LibraryView) {
		libraryView = view;
		if (initializedFromSettings) {
			settingsStore.update({ default_library_view: libraryView });
		}
	}

	let filtered = $derived(
		items.filter((a) => {
			if (
				debouncedSearch.trim() &&
				!a.title.toLowerCase().includes(debouncedSearch.trim().toLowerCase())
			) {
				return false;
			}
			if (libraryFiltersStore.sourceName && a.source_name !== libraryFiltersStore.sourceName) {
				return false;
			}
			if (
				libraryFiltersStore.tags.length &&
				!a.tags.some((tag) => libraryFiltersStore.tags.includes(tag))
			) {
				return false;
			}
			return true;
		})
	);

	// Mobile-only horizontal category-chip row (the sidebar's Categories
	// section has no equivalent on narrow screens) — drawn from the whole
	// library, not just this view's `items`, matching the design's global
	// category list.
	let allCategories = $derived(deriveCategories(articlesStore.items));
</script>

<div class="collection-page">
	<div class="header-row">
		<div>
			<h1>{title}</h1>
			{#if subtitle}<span class="sub">{subtitle}</span>{/if}
		</div>
		<div class="header-controls">
			<div class="search-box">
				<Search />
				<input type="text" placeholder="Search" bind:value={search} />
			</div>
			{#if showRefresh}
				<button
					class="btn btn-icon btn-secondary"
					onclick={() => sourcesStore.syncAll()}
					disabled={uiStore.syncing}
					aria-label="Refresh"
				>
					<Refresh spinning={uiStore.syncing} />
				</button>
			{/if}
			<div class="seg view-toggle">
				<label class="seg-opt">
					<input
						type="radio"
						name="libview-{title}"
						checked={libraryView === 'cards'}
						onchange={() => setView('cards')}
					/>
					<Grid />
				</label>
				<label class="seg-opt">
					<input
						type="radio"
						name="libview-{title}"
						checked={libraryView === 'list'}
						onchange={() => setView('list')}
					/>
					<ListIcon />
				</label>
			</div>
		</div>
	</div>

	{#if allCategories.length > 0}
		<div class="mobile-chips">
			<button
				class="chip"
				class:active={!libraryFiltersStore.sourceName}
				onclick={() => (libraryFiltersStore.sourceName = null)}
			>
				All
			</button>
			{#each allCategories as [name] (name)}
				<button
					class="chip"
					class:active={libraryFiltersStore.sourceName === name}
					onclick={() => libraryFiltersStore.toggleSource(name)}
				>
					{name}
				</button>
			{/each}
		</div>
	{/if}

	{#if items.length === 0}
		<p class="empty-state text-muted">{emptyMessage}</p>
	{:else if filtered.length === 0}
		<p class="empty-state text-muted">No articles match.</p>
	{:else if libraryView === 'cards'}
		<div class="cards-grid">
			{#each filtered as article (article.id)}
				<ArticleCard {article} onclick={() => onopen(article.id)} ondelete={() => ondelete(article)} />
			{/each}
		</div>
	{:else}
		<div class="list-rows">
			{#each filtered as article (article.id)}
				<ArticleListRow
					{article}
					onclick={() => onopen(article.id)}
					ondelete={() => ondelete(article)}
				/>
			{/each}
		</div>
	{/if}
</div>

<style>
	.collection-page {
		padding: 36px 36px 56px;
	}
	.header-row {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 16px;
		flex-wrap: wrap;
		margin-bottom: 22px;
	}
	.header-row h1 {
		font-size: 24px;
	}
	.sub {
		font-size: 12px;
		color: var(--color-muted);
	}
	.header-controls {
		display: flex;
		align-items: center;
		gap: 8px;
	}
	.search-box {
		display: flex;
		align-items: center;
		gap: 8px;
		background: var(--color-surface);
		border-radius: 12px;
		padding: 9px 14px;
		width: 220px;
		color: var(--color-muted);
	}
	.search-box input {
		border: none;
		background: transparent;
		outline: none;
		font: inherit;
		font-size: 13px;
		width: 100%;
		color: var(--color-text);
	}
	.view-toggle {
		border-radius: 10px;
	}
	.view-toggle .seg-opt {
		padding: 8px 10px;
	}
	.view-toggle :global(svg) {
		display: block;
	}
	.cards-grid {
		display: grid;
		/* A fixed max (not 1fr) keeps cards at a comfortable, constant size
		   as the window is resized — more columns appear as it widens,
		   rather than a couple of cards stretching to fill a narrow
		   window and towering into oversized squares. */
		grid-template-columns: repeat(auto-fill, minmax(216px, 264px));
		gap: 18px;
	}
	.list-rows {
		display: flex;
		flex-direction: column;
		gap: 2px;
	}
	.empty-state {
		padding: 40px 0;
	}

	.mobile-chips {
		display: none;
	}

	@media (max-width: 768px) {
		.collection-page {
			padding: 20px 16px 32px;
		}
		.search-box {
			width: 140px;
		}
		.mobile-chips {
			display: flex;
			gap: 6px;
			overflow-x: auto;
			margin-bottom: 16px;
			padding-bottom: 2px;
		}
		.chip {
			flex: none;
			font-size: 11.5px;
			padding: 6px 13px;
			border-radius: 999px;
			background: var(--color-surface);
			color: var(--color-text);
			border: none;
			cursor: pointer;
			white-space: nowrap;
		}
		.chip.active {
			background: var(--color-accent);
			color: var(--color-accent-fg);
		}
	}
</style>
