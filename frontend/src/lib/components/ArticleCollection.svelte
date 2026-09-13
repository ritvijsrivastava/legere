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

	// ── Virtualized rendering ───────────────────────────────────────────
	// With hundreds (soon thousands) of articles, mounting a live
	// ArticleCard/ArticleListRow — each with its own hero-image fetch and
	// CSS transitions — for every item regardless of scroll position is
	// the main source of jank. Instead we render only the rows within (or
	// just outside) the viewport, plus two spacer elements that stand in
	// for the rows above/below so the scrollbar's size/position stays
	// correct. `.scroll-area` below is this component's own scroll
	// container (rather than relying on the app shell's ancestor `.content`
	// pane) so the scroll position and the grid's content start at the
	// same coordinate — no ancestor-offset math needed. This also makes
	// the header/search bar sticky above the scrolling list, which reads
	// as an improvement in its own right for a long list.
	const OVERSCAN_ROWS = 3;
	// Keep in sync with the `gap`/`minmax()` values in the corresponding
	// CSS rules below — used to convert a measured card/row height into a
	// "pixels advanced per row" figure and back.
	const GRID_GAP = 18;
	const LIST_GAP = 2;
	const CARD_MIN_WIDTH = 216;

	let scrollAreaEl = $state<HTMLElement | null>(null);
	let gridEl = $state<HTMLElement | null>(null);
	let scrollTop = $state(0);
	let viewportHeight = $state(0);
	let containerWidth = $state(0);

	// Estimates until the real thing is measured (see the ResizeObserver
	// below) — only affects the very first frame after the grid/list
	// mounts.
	let rowHeightCards = $state(340);
	let rowHeightList = $state(80);

	$effect(() => {
		if (!scrollAreaEl) return;
		const el = scrollAreaEl;
		function onScroll() {
			scrollTop = el.scrollTop;
		}
		el.addEventListener('scroll', onScroll, { passive: true });
		const ro = new ResizeObserver(() => {
			viewportHeight = el.clientHeight;
		});
		ro.observe(el);
		onScroll();
		viewportHeight = el.clientHeight;
		return () => {
			el.removeEventListener('scroll', onScroll);
			ro.disconnect();
		};
	});

	$effect(() => {
		if (!gridEl) return;
		const el = gridEl;
		const ro = new ResizeObserver(() => {
			containerWidth = el.clientWidth;
		});
		ro.observe(el);
		containerWidth = el.clientWidth;
		return () => ro.disconnect();
	});

	// Snap back to the top whenever the result set is narrowed (or the
	// view mode changes row shape) — otherwise a mid-list scroll position
	// can point past the new, possibly much shorter, filtered set and the
	// virtualized window renders nothing visible.
	$effect(() => {
		void debouncedSearch;
		void libraryFiltersStore.sourceName;
		void libraryFiltersStore.tags;
		void libraryView;
		if (scrollAreaEl) scrollAreaEl.scrollTop = 0;
		scrollTop = 0;
	});

	let firstItemEl = $state<HTMLElement | null>(null);
	function bindFirstItem(el: HTMLElement) {
		firstItemEl = el;
	}
	$effect(() => {
		if (!firstItemEl) return;
		const el = firstItemEl;
		const measure = () => {
			const height = el.getBoundingClientRect().height;
			if (height <= 0) return;
			if (libraryView === 'cards') rowHeightCards = height + GRID_GAP;
			else rowHeightList = height + LIST_GAP;
		};
		measure();
		const ro = new ResizeObserver(measure);
		ro.observe(el);
		return () => ro.disconnect();
	});

	let columns = $derived(
		libraryView === 'cards' && containerWidth > 0
			? Math.max(1, Math.floor((containerWidth + GRID_GAP) / (CARD_MIN_WIDTH + GRID_GAP)))
			: 1
	);
	let rowHeight = $derived(libraryView === 'cards' ? rowHeightCards : rowHeightList);
	let rowGap = $derived(libraryView === 'cards' ? GRID_GAP : LIST_GAP);
	let totalRows = $derived(Math.max(1, Math.ceil(filtered.length / columns)));
	let startRow = $derived(Math.max(0, Math.floor(scrollTop / rowHeight) - OVERSCAN_ROWS));
	let endRow = $derived(
		Math.min(totalRows, Math.ceil((scrollTop + viewportHeight) / rowHeight) + OVERSCAN_ROWS)
	);
	let startIndex = $derived(startRow * columns);
	let endIndex = $derived(Math.min(filtered.length, endRow * columns));
	let visibleItems = $derived(filtered.slice(startIndex, endIndex));
	// The grid/flex `gap` is inserted structurally by the browser between
	// every adjacent row, including right after a spacer — so a spacer
	// standing in for N rows only needs N-1 gaps' worth of height itself
	// (its own trailing gap is supplied by the layout, not by us). Hence
	// the `- rowGap`, and skipping the spacer entirely (see the template)
	// when there's nothing to stand in for.
	let topSpacerHeight = $derived(Math.max(0, startRow * rowHeight - rowGap));
	let bottomSpacerHeight = $derived(Math.max(0, (totalRows - endRow) * rowHeight - rowGap));
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

	<div class="scroll-area" bind:this={scrollAreaEl}>
		{#if items.length === 0}
			<p class="empty-state text-muted">{emptyMessage}</p>
		{:else if filtered.length === 0}
			<p class="empty-state text-muted">No articles match.</p>
		{:else if libraryView === 'cards'}
			<div
				class="cards-grid"
				bind:this={gridEl}
				style:grid-template-columns={containerWidth > 0
					? `repeat(${columns}, minmax(${CARD_MIN_WIDTH}px, 264px))`
					: undefined}
			>
				{#if startRow > 0}
					<div class="v-spacer" style:height="{topSpacerHeight}px" style:grid-column="1 / -1"></div>
				{/if}
				{#each visibleItems as article, i (article.id)}
					<ArticleCard
						{article}
						onclick={() => onopen(article.id)}
						ondelete={() => ondelete(article)}
						onMountRoot={i === 0 ? bindFirstItem : undefined}
					/>
				{/each}
				{#if endRow < totalRows}
					<div
						class="v-spacer"
						style:height="{bottomSpacerHeight}px"
						style:grid-column="1 / -1"
					></div>
				{/if}
			</div>
		{:else}
			<div class="list-rows" bind:this={gridEl}>
				{#if startRow > 0}
					<div class="v-spacer" style:height="{topSpacerHeight}px"></div>
				{/if}
				{#each visibleItems as article, i (article.id)}
					<ArticleListRow
						{article}
						onclick={() => onopen(article.id)}
						ondelete={() => ondelete(article)}
						onMountRoot={i === 0 ? bindFirstItem : undefined}
					/>
				{/each}
				{#if endRow < totalRows}
					<div class="v-spacer" style:height="{bottomSpacerHeight}px"></div>
				{/if}
			</div>
		{/if}
	</div>
</div>

<style>
	.collection-page {
		display: flex;
		flex-direction: column;
		height: 100%;
		min-height: 0;
		padding: 36px 36px 0;
	}
	.header-row {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 16px;
		flex-wrap: wrap;
		margin-bottom: 22px;
		flex: none;
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
	.scroll-area {
		flex: 1;
		min-height: 0;
		overflow-y: auto;
		padding-bottom: 56px;
	}
	.cards-grid {
		display: grid;
		/* A fixed max (not 1fr) keeps cards at a comfortable, constant size
		   as the window is resized — more columns appear as it widens,
		   rather than a couple of cards stretching to fill a narrow
		   window and towering into oversized squares. The column *count*
		   is normally pinned by JS (see `columns` above) to match what
		   `auto-fill` would have picked, so the visible virtualized slice
		   lines up with the actual layout; `auto-fill` here is only a
		   same-looking fallback for the first frame before that
		   measurement lands. */
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
			padding: 20px 16px 0;
		}
		.scroll-area {
			padding-bottom: 32px;
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
			flex: none;
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
