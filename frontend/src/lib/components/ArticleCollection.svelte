<script lang="ts">
	import { goto } from '$app/navigation';
	import { settingsStore } from '$lib/stores/settings.svelte';
	import { sourcesStore } from '$lib/stores/sources.svelte';
	import { uiStore } from '$lib/stores/ui.svelte';
	import { libraryStatsStore } from '$lib/stores/libraryStats.svelte';
	import { libraryFiltersStore } from '$lib/stores/libraryFilters.svelte';
	import { sourceDotColor } from '$lib/sourceColor';
	import * as api from '$lib/api';
	import ArticleCard from './ArticleCard.svelte';
	import ArticleListRow from './ArticleListRow.svelte';
	import SearchScopeFilter from './SearchScopeFilter.svelte';
	import Search from '$lib/icons/Search.svelte';
	import Grid from '$lib/icons/Grid.svelte';
	import ListIcon from '$lib/icons/ListIcon.svelte';
	import Refresh from '$lib/icons/Refresh.svelte';
	import type { ArticleSummary, LibraryView, SearchScope } from '$lib/types';
	import type { Snippet } from 'svelte';

	let {
		title,
		subtitle,
		favoritedOnly = false,
		emptyMessage,
		showRefresh = false,
		hideCategoryChips = false,
		enableTypeSearch = false,
		onopen,
		headerActions
	}: {
		title: string;
		subtitle?: string;
		/** Scopes every fetch to `favorited = 1` (the Favorites view) rather
		 *  than the whole library. */
		favoritedOnly?: boolean;
		emptyMessage: string;
		showRefresh?: boolean;
		/** Hides the mobile category-chip row — used by the dedicated
		 *  `/category/[id]` page, where switching category via a chip would
		 *  silently desync the page's title/settings button from what's
		 *  actually being shown. */
		hideCategoryChips?: boolean;
		/** Extends the search box to also match category/tag names (not just
		 *  article titles) and renders them as grouped result sections below
		 *  the article list, with a scope filter to search only one kind.
		 *  Only enabled on the main Library view — `/favorites` and
		 *  `/category/[id]` are already scoped to one slice of the library,
		 *  where matching *other* categories/tags would be confusing. */
		enableTypeSearch?: boolean;
		onopen: (id: string) => void;
		/** Extra controls rendered at the end of the header row, e.g. the
		 *  category page's settings button. */
		headerActions?: Snippet;
	} = $props();

	let search = $state('');
	// The paginated fetch below re-queries on every dependency change —
	// debounce so fast typing doesn't fire a request per keystroke.
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

	let hasActiveFilters = $derived(
		debouncedSearch.trim().length > 0 ||
			libraryFiltersStore.categoryId !== null ||
			libraryFiltersStore.tags.length > 0
	);

	// ── Search scope (articles/categories/tags) ────────────────────────
	// Only meaningful once `enableTypeSearch` is on and there's an actual
	// query typed — with an empty box, or on a view that hasn't opted in,
	// the article grid always shows its normal unfiltered/filtered-by-
	// sidebar contents regardless of `searchScope`.
	let searchScope = $state<SearchScope>('all');
	let searchActive = $derived(enableTypeSearch && debouncedSearch.trim().length > 0);
	let showArticlesSection = $derived(
		!searchActive || searchScope === 'all' || searchScope === 'articles'
	);
	let showCategoriesSection = $derived(
		searchActive && (searchScope === 'all' || searchScope === 'categories')
	);
	let showTagsSection = $derived(
		searchActive && (searchScope === 'all' || searchScope === 'tags')
	);
	// Categories/tags are already fetched in full (with live counts) for
	// the sidebar — see `libraryStatsStore` — so matching them against the
	// query is just an in-memory filter, no extra round-trip needed.
	let matchedCategories = $derived(
		showCategoriesSection
			? libraryStatsStore.categories.filter((c) =>
					c.name.toLowerCase().includes(debouncedSearch.trim().toLowerCase())
				)
			: []
	);
	let matchedTags = $derived(
		showTagsSection
			? libraryStatsStore.tags.filter(([tag]) =>
					tag.toLowerCase().includes(debouncedSearch.trim().toLowerCase())
				)
			: []
	);

	function openMatchedCategory(id: string) {
		goto(`/category/${id}`);
	}

	/** Selecting a matched tag hands off to the sidebar's existing tag
	 *  filter (also visible/toggleable there) and clears the search box —
	 *  the typed query has done its job of finding the tag, the filter
	 *  chip takes it from here. */
	function selectMatchedTag(tag: string) {
		libraryFiltersStore.toggleTag(tag);
		search = '';
		debouncedSearch = '';
	}

	// ── Paginated data ──────────────────────────────────────────────────
	// Every filter (search/category/tags/favorited-only) is applied
	// server-side (see `ArticlePageRequest`) — this component only ever
	// holds however much of the *current* query's result set has been
	// scrolled into (see the virtualization section below), never the
	// whole library.
	const PAGE_SIZE = 60;
	let loadedItems = $state<ArticleSummary[]>([]);
	let nextCursor = $state<[string, string] | null>(null);
	let hasMore = $state(true);
	let loadingMore = $state(false);
	let initialLoading = $state(true);
	// Guards against a slow, now-superseded request (e.g. the previous
	// search term) overwriting the result of a newer one.
	let loadSeq = 0;

	function baseRequestFields() {
		return {
			limit: PAGE_SIZE,
			// Suppressed entirely (not just left unfiltered) when the search
			// scope excludes articles — see `showArticlesSection`.
			search: showArticlesSection ? debouncedSearch.trim() || null : null,
			category_id: libraryFiltersStore.categoryId,
			tags: [...libraryFiltersStore.tags],
			favorited_only: favoritedOnly
		};
	}

	async function loadFirstPage() {
		if (!showArticlesSection) {
			// A categories/tags-only search: no article query at all, just
			// invalidate whatever's in flight and clear the grid.
			++loadSeq;
			loadedItems = [];
			hasMore = false;
			nextCursor = null;
			initialLoading = false;
			return;
		}
		const seq = ++loadSeq;
		initialLoading = true;
		try {
			const page = await api.listArticlesPage({
				cursor_fetched_at: null,
				cursor_id: null,
				...baseRequestFields()
			});
			if (seq !== loadSeq) return;
			loadedItems = page.items;
			hasMore = page.has_more;
			nextCursor = page.next_cursor;
		} finally {
			if (seq === loadSeq) initialLoading = false;
		}
	}

	async function loadNextPage() {
		if (loadingMore || !hasMore || !nextCursor) return;
		const seq = loadSeq;
		loadingMore = true;
		try {
			const page = await api.listArticlesPage({
				cursor_fetched_at: nextCursor[0],
				cursor_id: nextCursor[1],
				...baseRequestFields()
			});
			if (seq !== loadSeq) return;
			loadedItems = [...loadedItems, ...page.items];
			hasMore = page.has_more;
			nextCursor = page.next_cursor;
		} finally {
			if (seq === loadSeq) loadingMore = false;
		}
	}

	// No explicit `libraryStatsStore.refresh()` here — `delete_article`
	// already emits `articles:changed`, which both refreshes the sidebar
	// stats and (via `changeVersion`, see below) lets *other* open views
	// notice; this view already knows (it just did the deleting).
	async function handleDelete(article: ArticleSummary) {
		if (!confirm(`Delete "${article.title}"? This can't be undone.`)) return;
		await api.deleteArticle(article.id);
		loadedItems = loadedItems.filter((a) => a.id !== article.id);
	}

	// Opens the shared `MoveToCategoryDialog` (mounted once in the root
	// layout). When this view is scoped to a single category (either the
	// dedicated category page or a sidebar/chip filter), a successful move
	// necessarily takes the article out of that scope, so it's dropped from
	// `loadedItems` immediately rather than waiting for the next
	// `changeVersion` merge (which only ever adds rows, never removes
	// stale ones — see `mergeInFreshFirstPage` above).
	// Mirrors `handleMoveToCategory` below — in the Favorites view, an
	// unfavorited article no longer belongs in `loadedItems` and won't
	// self-correct until the next `changeVersion` merge (which only adds
	// rows, never removes them), so drop it immediately.
	function handleFavoriteToggled(article: ArticleSummary) {
		if (favoritedOnly && !article.favorited) {
			loadedItems = loadedItems.filter((a) => a.id !== article.id);
		}
	}

	function handleMoveToCategory(article: ArticleSummary) {
		uiStore.openMoveCategory({ id: article.id, title: article.title }, () => {
			if (libraryFiltersStore.categoryId !== null) {
				loadedItems = loadedItems.filter((a) => a.id !== article.id);
			}
		});
	}

	// Re-query from the top whenever a filter (or the favorited-only scope
	// itself) changes — a fundamentally different result set, so resetting
	// scroll to the top makes sense here.
	$effect(() => {
		void debouncedSearch;
		void libraryFiltersStore.categoryId;
		void libraryFiltersStore.tags;
		void favoritedOnly;
		void showArticlesSection;
		if (scrollAreaEl) scrollAreaEl.scrollTop = 0;
		scrollTop = 0;
		loadFirstPage();
	});

	// Switching card/list view doesn't need a re-query (same data), just a
	// scroll reset (row height differs between the two).
	$effect(() => {
		void libraryView;
		if (scrollAreaEl) scrollAreaEl.scrollTop = 0;
		scrollTop = 0;
	});

	// The backend reports a library change from *anywhere* (sync, import,
	// a delete from another view, etc.) via one coarse `articles:changed`
	// event bumping `libraryStatsStore.changeVersion` — unlike the filter
	// effect above, this must NOT reset scroll or fully reload (the user
	// could be mid-scroll through what's already loaded, and a sync
	// tick shouldn't yank them back to the top). Instead, quietly fetch a
	// fresh page 1 and prepend whatever's genuinely new (new articles sort
	// first, so the fresh page's order is already the correct prepend
	// order) — anything already loaded is left exactly where it is.
	let changeVersionInitialized = false;
	$effect(() => {
		const version = libraryStatsStore.changeVersion;
		void version;
		if (!changeVersionInitialized) {
			// Skip the run every `$effect` does immediately on mount — this
			// is for *subsequent* external changes only; the initial load is
			// already `loadFirstPage`'s job.
			changeVersionInitialized = true;
			return;
		}
		mergeInFreshFirstPage();
	});

	async function mergeInFreshFirstPage() {
		if (initialLoading || !showArticlesSection) return;
		try {
			const page = await api.listArticlesPage({
				cursor_fetched_at: null,
				cursor_id: null,
				...baseRequestFields()
			});
			const existingIds = new Set(loadedItems.map((a) => a.id));
			const newOnes = page.items.filter((a) => !existingIds.has(a.id));
			if (newOnes.length > 0) {
				loadedItems = [...newOnes, ...loadedItems];
			}
			// Nothing was loaded to merge into (e.g. every previously loaded
			// row was deleted elsewhere) — fall back to adopting the fresh
			// page wholesale so `hasMore`/`nextCursor` don't stay stuck.
			if (loadedItems.length === 0) {
				loadedItems = page.items;
				hasMore = page.has_more;
				nextCursor = page.next_cursor;
			}
		} catch {
			// Best-effort background refresh — a failure here shouldn't
			// disrupt whatever's already on screen.
		}
	}

	// ── Virtualized rendering ───────────────────────────────────────────
	// With hundreds (soon thousands) of articles, mounting a live
	// ArticleCard/ArticleListRow — each with its own hero-image fetch and
	// CSS transitions — for every loaded item regardless of scroll
	// position is the main source of jank. Instead we render only the
	// rows within (or just outside) the viewport, plus two spacer
	// elements that stand in for the rows above/below so the scrollbar's
	// size/position stays correct. `.scroll-area` below is this
	// component's own scroll container (rather than relying on the app
	// shell's ancestor `.content` pane) so the scroll position and the
	// grid's content start at the same coordinate — no ancestor-offset
	// math needed. This also makes the header/search bar sticky above the
	// scrolling list, which reads as an improvement in its own right for
	// a long list. The same viewport tracking also drives when to fetch
	// the next page (see the effect at the bottom of this section).
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
	let totalRows = $derived(Math.max(1, Math.ceil(loadedItems.length / columns)));
	let startRow = $derived(Math.max(0, Math.floor(scrollTop / rowHeight) - OVERSCAN_ROWS));
	let endRow = $derived(
		Math.min(totalRows, Math.ceil((scrollTop + viewportHeight) / rowHeight) + OVERSCAN_ROWS)
	);
	let startIndex = $derived(startRow * columns);
	let endIndex = $derived(Math.min(loadedItems.length, endRow * columns));
	let visibleItems = $derived(loadedItems.slice(startIndex, endIndex));
	// The grid/flex `gap` is inserted structurally by the browser between
	// every adjacent row, including right after a spacer — so a spacer
	// standing in for N rows only needs N-1 gaps' worth of height itself
	// (its own trailing gap is supplied by the layout, not by us). Hence
	// the `- rowGap`, and skipping the spacer entirely (see the template)
	// when there's nothing to stand in for.
	let topSpacerHeight = $derived(Math.max(0, startRow * rowHeight - rowGap));
	let bottomSpacerHeight = $derived(Math.max(0, (totalRows - endRow) * rowHeight - rowGap));

	// Fetch the next page once the virtualized window's range comes
	// within a page-sized buffer of the end of what's currently loaded.
	$effect(() => {
		if (initialLoading || loadingMore || !hasMore) return;
		if (totalRows - endRow <= OVERSCAN_ROWS) {
			loadNextPage();
		}
	});
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
				<input
					type="text"
					placeholder="Search"
					bind:value={search}
					spellcheck="false"
					autocomplete="off"
					autocorrect="off"
					autocapitalize="off"
				/>
				{#if enableTypeSearch}
					<SearchScopeFilter bind:scope={searchScope} />
				{/if}
			</div>
			{#if headerActions}{@render headerActions()}{/if}
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

	{#if !hideCategoryChips && libraryStatsStore.categories.length > 0}
		<div class="mobile-chips">
			<button
				class="chip"
				class:active={!libraryFiltersStore.categoryId}
				onclick={() => (libraryFiltersStore.categoryId = null)}
			>
				All
			</button>
			{#each libraryStatsStore.categories as category (category.id)}
				<button
					class="chip"
					class:active={libraryFiltersStore.categoryId === category.id}
					onclick={() => libraryFiltersStore.toggleCategory(category.id)}
				>
					{category.name}
				</button>
			{/each}
		</div>
	{/if}

	<div class="scroll-area" bind:this={scrollAreaEl}>
		{#if showArticlesSection}
		{#if initialLoading}
			<!-- Nothing yet — avoids a flash of `emptyMessage` while the
			     first page is still in flight. -->
		{:else if loadedItems.length === 0 && !hasActiveFilters}
			<p class="empty-state text-muted">{emptyMessage}</p>
		{:else if loadedItems.length === 0}
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
						ondelete={() => handleDelete(article)}
						onmove={() => handleMoveToCategory(article)}
						onfavorite={() => handleFavoriteToggled(article)}
						onMountRoot={startIndex + i === 0 ? bindFirstItem : undefined}
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
						ondelete={() => handleDelete(article)}
						onmove={() => handleMoveToCategory(article)}
						onfavorite={() => handleFavoriteToggled(article)}
						onMountRoot={startIndex + i === 0 ? bindFirstItem : undefined}
					/>
				{/each}
				{#if endRow < totalRows}
					<div class="v-spacer" style:height="{bottomSpacerHeight}px"></div>
				{/if}
			</div>
		{/if}
		{/if}

		{#if showCategoriesSection}
			<section class="type-results">
				<h2 class="type-results-heading">Categories</h2>
				{#if matchedCategories.length === 0}
					<p class="empty-state text-muted small">No matching categories.</p>
				{:else}
					<div class="matched-categories">
						{#each matchedCategories as category (category.id)}
							<button class="matched-row" onclick={() => openMatchedCategory(category.id)}>
								<span class="dot" style:background={sourceDotColor(category.name)}></span>
								<span class="row-label">{category.name}</span>
								<span class="row-count">{category.article_count}</span>
							</button>
						{/each}
					</div>
				{/if}
			</section>
		{/if}

		{#if showTagsSection}
			<section class="type-results">
				<h2 class="type-results-heading">Tags</h2>
				{#if matchedTags.length === 0}
					<p class="empty-state text-muted small">No matching tags.</p>
				{:else}
					<div class="matched-tags">
						{#each matchedTags as [tag, count] (tag)}
							<button
								class="tag-chip"
								class:active={libraryFiltersStore.tags.includes(tag)}
								onclick={() => selectMatchedTag(tag)}
							>
								#{tag}
								<span class="row-count">{count}</span>
							</button>
						{/each}
					</div>
				{/if}
			</section>
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
		padding: 9px 10px 9px 14px;
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
	.empty-state.small {
		padding: 4px 0 8px;
		font-size: 12.5px;
	}

	/* ── Categories/Tags search results ─────────────────────────────── */
	.type-results {
		margin-top: 28px;
		padding-top: 20px;
		border-top: 1px solid var(--color-divider);
	}
	.type-results:first-child {
		margin-top: 0;
		padding-top: 0;
		border-top: none;
	}
	.type-results-heading {
		font-size: 10.5px;
		letter-spacing: 0.06em;
		text-transform: uppercase;
		color: var(--color-muted);
		margin: 0 0 10px;
		font-family: var(--font-body);
		font-weight: 600;
	}
	.matched-categories {
		display: flex;
		flex-direction: column;
		gap: 2px;
	}
	.matched-row {
		display: flex;
		align-items: center;
		gap: 10px;
		width: 100%;
		text-align: left;
		background: none;
		border: none;
		border-radius: var(--radius-md);
		padding: 9px 10px;
		font-family: var(--font-body);
		font-size: 13.5px;
		color: var(--color-text);
		cursor: pointer;
	}
	.matched-row:hover {
		background: var(--color-surface);
	}
	.matched-row .dot {
		width: 7px;
		height: 7px;
		border-radius: 50%;
		flex: none;
	}
	.matched-row .row-label {
		flex: 1;
	}
	.row-count {
		font-size: 11px;
		color: var(--color-muted);
	}
	.matched-tags {
		display: flex;
		flex-wrap: wrap;
		gap: 8px;
	}
	.matched-tags .tag-chip {
		display: flex;
		align-items: center;
		gap: 6px;
		font-size: 12px;
		padding: 7px 13px;
		border-radius: 999px;
		background: var(--color-surface);
		color: var(--color-text);
		border: none;
		cursor: pointer;
		font-family: var(--font-body);
	}
	.matched-tags .tag-chip.active {
		background: var(--color-accent);
		color: var(--color-accent-fg);
	}
	.matched-tags .tag-chip.active .row-count {
		color: inherit;
		opacity: 0.8;
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
