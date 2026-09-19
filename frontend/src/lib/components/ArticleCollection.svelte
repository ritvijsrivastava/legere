<script lang="ts">
	import { goto } from '$app/navigation';
	import { settingsStore } from '$lib/stores/settings.svelte';
	import { sourcesStore } from '$lib/stores/sources.svelte';
	import { uiStore } from '$lib/stores/ui.svelte';
	import { libraryStatsStore } from '$lib/stores/libraryStats.svelte';
	import { libraryFiltersStore } from '$lib/stores/libraryFilters.svelte';
	import * as api from '$lib/api';
	import ArticleCard from './ArticleCard.svelte';
	import ArticleListRow from './ArticleListRow.svelte';
	import CategoryIcon from './CategoryIcon.svelte';
	import SearchScopeFilter from './SearchScopeFilter.svelte';
	import MobileTagSheet from './MobileTagSheet.svelte';
	import MobileCategorySheet from './MobileCategorySheet.svelte';
	import Search from '$lib/icons/Search.svelte';
	import Grid from '$lib/icons/Grid.svelte';
	import ListIcon from '$lib/icons/ListIcon.svelte';
	import Refresh from '$lib/icons/Refresh.svelte';
	import Hash from '$lib/icons/Hash.svelte';
	import Folder from '$lib/icons/Folder.svelte';
	import X from '$lib/icons/X.svelte';
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

	// Whether the mobile category-chip row (folders) and the mobile Tags
	// sheet's own trigger chip should render — independent conditions, so
	// a page that hides category chips (the dedicated `/category/[id]`
	// page) can still offer tag filtering, and a library with categories
	// but no tags yet doesn't show an empty Tags trigger.
	let showCategoryChips = $derived(!hideCategoryChips && libraryStatsStore.categories.length > 0);
	let showTagsChip = $derived(libraryStatsStore.tags.length > 0);
	let tagSheetOpen = $state(false);
	let categorySheetOpen = $state(false);
	// The active category (if any) shown as its own removable chip next to
	// the "Categories" trigger — mirrors how an active tag renders (trigger
	// chip + one removable chip per selection), so a single chosen category
	// reads the same way a single chosen tag does.
	let activeCategory = $derived(
		libraryFiltersStore.categoryId
			? (libraryStatsStore.categories.find((c) => c.id === libraryFiltersStore.categoryId) ?? null)
			: null
	);

	// ── Mobile search reveal ────────────────────────────────────────────
	// The permanent search bar reads as a lot of chrome for something used
	// occasionally — on mobile it collapses to a single "Search" button;
	// tapping it reveals the real input (autofocused) on its own row, and
	// closing it clears the query rather than just hiding a stale one.
	let mobileSearchOpen = $state(false);
	let mobileSearchInputEl = $state<HTMLInputElement | null>(null);
	$effect(() => {
		if (mobileSearchOpen) mobileSearchInputEl?.focus();
	});
	function closeMobileSearch() {
		mobileSearchOpen = false;
		search = '';
	}

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
	const GRID_GAP_DESKTOP = 18;
	const GRID_GAP_MOBILE = 12;
	const LIST_GAP = 2;
	const CARD_MIN_WIDTH_DESKTOP = 216;
	const CARD_MAX_WIDTH_DESKTOP = 264;
	// A phone-width grid gets two touch-sized columns instead of one
	// desktop-sized one — the fixed-comfortable-width philosophy (see the
	// `.cards-grid` comment below) just needs a smaller comfortable size to
	// apply on a ~360–430px-wide screen.
	const CARD_MIN_WIDTH_MOBILE = 152;
	const CARD_MAX_WIDTH_MOBILE = 208;
	// Below this measured *grid* width (not viewport width — this already
	// excludes page padding, so it lines up with an actual phone screen),
	// favor the mobile card size. A container this narrow can't fit two
	// desktop-sized cards anyway, so this never fights the desktop tiers.
	const MOBILE_GRID_THRESHOLD = 620;

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

	// ── Pull-to-refresh (mobile, touch only) ──────────────────────────
	// Replaces the header's refresh button on mobile — only wired up when
	// `showRefresh` (only the Library view syncs sources) is true. Tracked
	// as a real height on a sibling of `.scroll-area` rather than a
	// transform, so it just pushes the (flex) scroll area down through
	// normal layout instead of needing to fake a gap above it.
	const PULL_MAX = 72;
	const PULL_THRESHOLD = 56;
	let pullY = $state(0);
	let pullTouchId = $state<number | null>(null);
	let isPulling = $derived(pullTouchId !== null);
	let pullStartY = 0;

	$effect(() => {
		if (!scrollAreaEl || !showRefresh) return;
		const el = scrollAreaEl;

		function onTouchStart(e: TouchEvent) {
			if (pullTouchId !== null || el.scrollTop > 0 || uiStore.syncing) return;
			const touch = e.touches[0];
			pullTouchId = touch.identifier;
			pullStartY = touch.clientY;
		}
		function activeTouch(e: TouchEvent) {
			return Array.from(e.touches).find((t) => t.identifier === pullTouchId);
		}
		function onTouchMove(e: TouchEvent) {
			if (pullTouchId === null) return;
			const touch = activeTouch(e);
			if (!touch || el.scrollTop > 0) {
				pullTouchId = null;
				pullY = 0;
				return;
			}
			const delta = touch.clientY - pullStartY;
			if (delta <= 0) {
				pullY = 0;
				return;
			}
			// Rubber-band damping — the further past the threshold, the less
			// additional travel each pixel of finger movement buys.
			pullY = Math.min(PULL_MAX, delta * 0.5);
			e.preventDefault();
		}
		async function onTouchEnd(e: TouchEvent) {
			if (pullTouchId === null) return;
			if (activeTouch(e)) return; // a *different* touch ended; ours is still down
			pullTouchId = null;
			if (pullY >= PULL_THRESHOLD) {
				pullY = PULL_THRESHOLD;
				// Resolved directly, rather than watching `uiStore.syncing` (the
				// same flag the desktop refresh button's spinner uses) — with no
				// syncable RSS sources, the backend never emits `sync:started`/
				// `sync:finished` at all (see `sync_all_sources`'s early return),
				// so that flag would never toggle and the indicator would stay
				// stuck open. Awaiting the call itself always resolves.
				try {
					await sourcesStore.syncAll();
				} finally {
					pullY = 0;
				}
			} else {
				pullY = 0;
			}
		}
		el.addEventListener('touchstart', onTouchStart, { passive: true });
		el.addEventListener('touchmove', onTouchMove, { passive: false });
		el.addEventListener('touchend', onTouchEnd, { passive: true });
		el.addEventListener('touchcancel', onTouchEnd, { passive: true });
		return () => {
			el.removeEventListener('touchstart', onTouchStart);
			el.removeEventListener('touchmove', onTouchMove);
			el.removeEventListener('touchend', onTouchEnd);
			el.removeEventListener('touchcancel', onTouchEnd);
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
			if (libraryView === 'cards') rowHeightCards = height + gridGap;
			else rowHeightList = height + LIST_GAP;
		};
		measure();
		const ro = new ResizeObserver(measure);
		ro.observe(el);
		return () => ro.disconnect();
	});

	let isNarrowGrid = $derived(containerWidth > 0 && containerWidth < MOBILE_GRID_THRESHOLD);
	let cardMinWidth = $derived(isNarrowGrid ? CARD_MIN_WIDTH_MOBILE : CARD_MIN_WIDTH_DESKTOP);
	let cardMaxWidth = $derived(isNarrowGrid ? CARD_MAX_WIDTH_MOBILE : CARD_MAX_WIDTH_DESKTOP);
	let gridGap = $derived(isNarrowGrid ? GRID_GAP_MOBILE : GRID_GAP_DESKTOP);
	let columns = $derived(
		libraryView === 'cards' && containerWidth > 0
			? Math.max(1, Math.floor((containerWidth + gridGap) / (cardMinWidth + gridGap)))
			: 1
	);
	let rowHeight = $derived(libraryView === 'cards' ? rowHeightCards : rowHeightList);
	let rowGap = $derived(libraryView === 'cards' ? gridGap : LIST_GAP);
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
	<div class="header-row" class:search-open={mobileSearchOpen}>
		<div class="title-block">
			<h1>{title}</h1>
			{#if subtitle}<span class="sub">{subtitle}</span>{/if}
		</div>
		<div class="header-controls">
			<div class="search-box" class:mobile-open={mobileSearchOpen}>
				<Search />
				<input
					type="text"
					placeholder="Search"
					bind:value={search}
					bind:this={mobileSearchInputEl}
					spellcheck="false"
					autocomplete="off"
					autocorrect="off"
					autocapitalize="off"
				/>
				{#if enableTypeSearch}
					<SearchScopeFilter bind:scope={searchScope} />
				{/if}
				{#if mobileSearchOpen}
					<button class="search-close-btn" onclick={closeMobileSearch} aria-label="Close search">
						<X size={15} />
					</button>
				{/if}
			</div>
			<div class="header-icons">
				<button
					class="btn btn-icon btn-secondary search-trigger-btn"
					onclick={() => (mobileSearchOpen = true)}
					aria-label="Search"
				>
					<Search size={15} />
				</button>
				{#if headerActions}{@render headerActions()}{/if}
				{#if showRefresh}
					<button
						class="btn btn-icon btn-secondary refresh-btn"
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
	</div>

	{#if showCategoryChips || showTagsChip}
		<div class="mobile-chips">
			{#if showCategoryChips}
				<button
					class="chip chip-categories"
					class:active={!!libraryFiltersStore.categoryId}
					onclick={() => (categorySheetOpen = true)}
				>
					<Folder size={11} />
					Categories
				</button>
				{#if activeCategory}
					<button
						class="chip chip-tag-active"
						onclick={() => (libraryFiltersStore.categoryId = null)}
					>
						{activeCategory.name}
						<X size={10} />
					</button>
				{/if}
			{/if}
			{#if showTagsChip}
				{#if showCategoryChips}<span class="chip-divider"></span>{/if}
				<button
					class="chip chip-tags"
					class:active={libraryFiltersStore.tags.length > 0}
					onclick={() => (tagSheetOpen = true)}
				>
					<Hash size={11} />
					Tags
				</button>
				{#each libraryFiltersStore.tags as tag (tag)}
					<button class="chip chip-tag-active" onclick={() => libraryFiltersStore.toggleTag(tag)}>
						#{tag}
						<X size={10} />
					</button>
				{/each}
			{/if}
		</div>
	{/if}

	<MobileTagSheet open={tagSheetOpen} onclose={() => (tagSheetOpen = false)} />
	<MobileCategorySheet open={categorySheetOpen} onclose={() => (categorySheetOpen = false)} />

	{#if showRefresh}
		<div class="pull-indicator" class:dragging={isPulling} style:height="{pullY}px">
			<Refresh size={16} spinning={uiStore.syncing} />
		</div>
	{/if}
	<div class="scroll-area" bind:this={scrollAreaEl}>
		<div class="scroll-inner">
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
					? `repeat(${columns}, minmax(${cardMinWidth}px, ${cardMaxWidth}px))`
					: undefined}
				style:gap="{gridGap}px"
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
								<CategoryIcon icon={category.icon} size={12} />
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
</div>

<style>
	.collection-page {
		display: flex;
		flex-direction: column;
		height: 100%;
		min-height: 0;
		/* No horizontal padding here — it lives on `.header-row`/`.mobile-chips`
		   and the new `.scroll-inner` wrapper below instead, so `.scroll-area`
		   itself stays full-bleed and its native scrollbar renders flush at
		   the real screen edge rather than inset by a parent's padding. */
		padding: 36px 0 0;
	}
	.header-row {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 16px;
		flex-wrap: wrap;
		margin-bottom: 22px;
		flex: none;
		padding: 0 36px;
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
	.header-icons {
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
	/* Mobile-only — collapses the search box into a single button (see
	   `.search-box`/`.search-trigger-btn` overrides in the mobile media
	   query below); hidden on desktop, which keeps the permanent inline
	   box. */
	.search-trigger-btn {
		display: none;
	}
	.search-close-btn {
		display: flex;
		flex: none;
		align-items: center;
		justify-content: center;
		background: none;
		border: none;
		color: var(--color-muted);
		cursor: pointer;
		padding: 4px;
	}
	.view-toggle {
		border-radius: 10px;
		/* Matches `.btn-icon`'s height exactly (see the mobile override below
		   too) so this pill reads as the same size class as the icon buttons
		   it sits next to in `.header-icons`, rather than a visibly shorter
		   control beside them. */
		height: 36px;
	}
	.view-toggle .seg-opt {
		height: 100%;
		display: flex;
		align-items: center;
		justify-content: center;
		padding: 0 10px;
	}
	.view-toggle :global(svg) {
		display: block;
	}
	.scroll-area {
		flex: 1;
		min-height: 0;
		overflow-y: auto;
	}
	/* Carries the horizontal reading margin that used to live on
	   `.collection-page` — kept on this inner wrapper rather than on
	   `.scroll-area` itself so the scrollbar isn't inset by it (see the
	   comment on `.collection-page`). Vertical padding stays a separate,
	   unpadded-horizontally concern of `.scroll-area` below. */
	.scroll-inner {
		padding: 0 36px 56px;
	}
	/* Grows/shrinks with the pull-to-refresh gesture (mobile only, see the
	   touch handlers above) — a real flex sibling of `.scroll-area` rather
	   than a transform, so it pushes the list down through normal layout
	   instead of needing to fake a gap above it. Renders (height: 0) on
	   every view with `showRefresh`, including desktop, but only a real
	   touch drag ever gives it height, so it's inert there. */
	.pull-indicator {
		flex: none;
		overflow: hidden;
		display: flex;
		align-items: center;
		justify-content: center;
		color: var(--color-muted);
		/* A real layout-property (`height`) animation, deliberately — this
		   box is standing in for actual displaced content (the flex sibling
		   `.scroll-area` below it shrinks to make room), which a transform
		   can't fake. Confined to the single release-triggered snap: turned
		   off entirely while a finger is actively dragging it (below), so it
		   never fights 60fps of direct per-frame updates. */
		transition: height 0.18s var(--ease-snap);
	}
	.pull-indicator.dragging {
		transition: none;
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
			padding: calc(20px + env(safe-area-inset-top)) 0 0;
		}
		/* 32px of normal breathing room plus enough extra to clear the
		   floating add-source FAB (see `Shell.svelte`), which overlays the
		   bottom-right corner of every mobile page — without this, its last
		   row/card would sit partly behind the button. */
		.scroll-inner {
			padding: 0 16px 104px;
		}

		/* Three clear rows instead of one crammed one: title + icons share
		   the top row (there's room — titles here are short), the search
		   field gets a full-width row of its own, and category/tag chips
		   scroll below. `.header-controls` goes `display: contents` so its
		   two children become direct participants in this grid rather than
		   both being pinned to one `header-controls`-sized box. */
		.header-row {
			display: grid;
			grid-template-columns: 1fr auto;
			grid-template-areas: 'title icons';
			column-gap: 12px;
			margin-bottom: 16px;
			padding: 0 16px;
		}
		/* Only reserves the second row (and its gap) while the search field
		   is actually open — otherwise collapsing it back to the trigger
		   button would still leave a phantom empty grid row/gap behind. */
		.header-row.search-open {
			grid-template-areas: 'title icons' 'search search';
			row-gap: 12px;
		}
		.title-block {
			grid-area: title;
			min-width: 0;
		}
		.header-controls {
			display: contents;
		}
		.header-icons {
			grid-area: icons;
		}
		.search-trigger-btn {
			display: flex;
		}
		/* `.btn-icon` itself grows to 44px under this same breakpoint (see
		   `components.css`) — mirror that here so the view-toggle pill stays
		   the same height as the search button beside it instead of a shorter
		   36px control next to a 44px one. */
		.view-toggle {
			height: 44px;
		}
		.refresh-btn {
			/* Replaced by the pull-to-refresh gesture on mobile (see the touch
			   handlers above) — a button and a pull gesture both wired to the
			   same action is redundant chrome on a touchscreen. */
			display: none;
		}
		/* Hidden by default — collapsed into `.search-trigger-btn` above.
		   `.mobile-open` reveals it on its own full-width row; an empty grid
		   row costs no space (see the `display: none` default), so there's
		   no gap left behind while it's collapsed. */
		/* The layout changer (card/list) is desktop-only chrome — on mobile
		   it moves to Settings (its own "Library" section already mirrors this
		   exact control), which both frees up header space for the search
		   button and keeps a rarely-touched preference out of the primary
		   navigation surface. */
		.view-toggle {
			display: none;
		}
		/* Now the last (often only) icon in `.header-icons`, so it already
		   lands at the row's right edge — just needs to read as "a bare icon",
		   not another bordered chip beside it. */
		.search-trigger-btn {
			border-color: transparent;
			background: transparent;
		}
		.search-box {
			display: none;
		}
		.search-box.mobile-open {
			grid-area: search;
			display: flex;
			width: 100%;
			height: 44px;
			border-radius: var(--radius-lg);
			padding: 0 12px 0 16px;
		}
		.search-box.mobile-open input {
			font-size: 15px;
		}
		.search-box.mobile-open :global(svg) {
			flex: none;
		}

		.mobile-chips {
			display: flex;
			align-items: center;
			gap: 6px;
			overflow-x: auto;
			margin-bottom: 16px;
			padding: 0 16px 2px;
			flex: none;
		}
		.chip {
			flex: none;
			display: flex;
			align-items: center;
			gap: 5px;
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
		.chip-divider {
			flex: none;
			width: 1px;
			align-self: stretch;
			margin: 4px 2px;
			background: var(--color-divider);
		}
		.chip-categories.active,
		.chip-tags.active {
			background: color-mix(in srgb, var(--color-accent) 16%, var(--color-surface));
			color: var(--color-accent);
		}
		.chip-tag-active {
			background: var(--color-accent);
			color: var(--color-accent-fg);
		}
	}
</style>
