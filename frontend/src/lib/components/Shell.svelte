<script lang="ts">
	import { page } from '$app/state';
	import Logo from '$lib/icons/Logo.svelte';
	import Library from '$lib/icons/Library.svelte';
	import Star from '$lib/icons/Star.svelte';
	import SettingsIcon from '$lib/icons/Settings.svelte';
	import Plus from '$lib/icons/Plus.svelte';
	import Search from '$lib/icons/Search.svelte';
	import X from '$lib/icons/X.svelte';
	import ChevronRight from '$lib/icons/ChevronRight.svelte';
	import { libraryStatsStore } from '$lib/stores/libraryStats.svelte';
	import { uiStore } from '$lib/stores/ui.svelte';
	import { libraryFiltersStore } from '$lib/stores/libraryFilters.svelte';
	import { sourceDotColor } from '$lib/sourceColor';
	import { goto } from '$app/navigation';
	import { isTauri } from '$lib/platform';
	import { checkForUpdate, getLastDismissedVersion, setLastDismissedVersion } from '$lib/update';
	import UpdateToast from '$lib/components/UpdateToast.svelte';
	import * as api from '$lib/api';
	import type { NamedCount } from '$lib/types';

	let { children } = $props();

	let isMobile = $state(typeof window !== 'undefined' ? window.innerWidth < 768 : false);
	let isReader = $derived(page.url.pathname.startsWith('/reader/'));
	let isSettings = $derived(page.url.pathname === '/settings');

	$effect(() => {
		function onResize() {
			isMobile = window.innerWidth < 768;
		}
		window.addEventListener('resize', onResize);
		return () => window.removeEventListener('resize', onResize);
	});

	// ── Background update check ──────────────────────────────────────────────
	let updateAvailable = $state<{ version: string; notes?: string | null } | null>(null);

	/** Fire-and-forget: check for an update without blocking or delaying page load.
	 *  Skipped when landing directly on Settings — that page runs its own check
	 *  on mount, and duplicating it here would just double the GitHub call. */
	async function backgroundCheckForUpdate() {
		if (!isTauri() || isSettings) return;
		try {
			const result = await checkForUpdate();
			if (!result.available) return;
			const dismissed = await getLastDismissedVersion();
			if (dismissed === result.version) return;
			updateAvailable = { version: result.version, notes: result.notes };
		} catch {
			// No token configured yet, or a transient network error — the user
			// can still check manually from Settings, so fail silently here.
		}
	}

	function dismissUpdate() {
		if (updateAvailable) setLastDismissedVersion(updateAvailable.version);
		updateAvailable = null;
	}

	const showUpdatePrompt = $derived(updateAvailable !== null && !isSettings);

	$effect(() => {
		backgroundCheckForUpdate();
	});

	const navItems = [
		{ href: '/', label: 'Library', Icon: Library, count: () => libraryStatsStore.totalCount },
		{ href: '/favorites', label: 'Favorites', Icon: Star, count: () => libraryStatsStore.favoritedCount },
		{ href: '/settings', label: 'Settings', Icon: SettingsIcon, count: null as (() => number) | null }
	];

	function isActive(href: string): boolean {
		return href === '/' ? page.url.pathname === '/' : page.url.pathname.startsWith(href);
	}

	function isActiveCategory(id: string): boolean {
		return page.url.pathname === `/category/${id}`;
	}

	// Sidebar Categories/Tags: categories are real user-managed folders.
	// Empty categories remain visible; Uncategorized is a virtual entry for
	// articles whose category_id is null. Both lists come from SQL-backed
	// aggregate stores rather than
	// scanning an in-memory copy of the whole library, which no longer
	// exists once articles are paginated (see `ArticleCollection`).
	let categories = $derived(libraryStatsStore.categories);
	let tags = $derived(libraryStatsStore.tags);

	// Tags section: collapsible (state survives navigation — this
	// component never remounts between routes), with its own name search
	// and a facet-narrowed list. Selecting a tag pins it above the list
	// (removable via its ✕) and re-scopes the rest of the list to only
	// tags that actually co-occur with the current selection—computed
	// server-side by `list_tags_filtered` using the exact same filters as
	// the article list itself (see `ARCHITECTURE.md`).
	let tagsOpen = $state(true);
	let tagSearch = $state('');
	let facetTags = $state<NamedCount[]>([]);

	$effect(() => {
		const categoryId = libraryFiltersStore.categoryId;
		const selected = libraryFiltersStore.tags;
		// Re-run on any tag/article mutation too (capture, edit, rename,
		// delete), not just when the sidebar's own filters change.
		void libraryStatsStore.changeVersion;

		// No active filter: `libraryStatsStore.tags` (already fetched —
		// it's also what gates this whole section's visibility below) is
		// exactly what `list_tags_filtered` would return for empty
		// filters. Reusing it directly means the default view never
		// depends on a second round trip landing before anything shows.
		if (!categoryId && selected.length === 0) {
			facetTags = libraryStatsStore.tags;
			return;
		}

		let cancelled = false;
		api
			.listTagsFiltered({ category_id: categoryId, tags: selected })
			.then((result) => {
				if (!cancelled) facetTags = result;
			})
			.catch(() => {
				// Best-effort UI narrowing — leave the previous list showing
				// rather than surface an error toast for a background refresh.
			});
		return () => {
			cancelled = true;
		};
	});

	// Already-selected tags are pinned separately above; the browsing list
	// below excludes them and applies the name search.
	let visibleTags = $derived(
		facetTags.filter(
			([tag]) =>
				!libraryFiltersStore.tags.includes(tag) && tag.includes(tagSearch.trim().toLowerCase())
		)
	);
</script>

{#snippet sidebarNav()}
	<nav class="nav-list">
		{#each navItems as item (item.href)}
			<a href={item.href} class="nav-item" class:active={isActive(item.href)}>
				<item.Icon size={16} />
				<span class="nav-label">{item.label}</span>
				{#if item.count}
					<span class="nav-count">{item.count()}</span>
				{/if}
			</a>
		{/each}
	</nav>

	{#if categories.length > 0}
		<div class="divider"></div>
		<div class="section-label">Categories</div>
		{#each categories as category (category.id)}
			<a
				href="/category/{category.id}"
				class="row-item"
				class:active={isActiveCategory(category.id)}
			>
				<span class="dot" style:background={sourceDotColor(category.name)}></span>
				<span class="row-label">{category.name}</span>
				<span class="nav-count">{category.article_count}</span>
			</a>
		{/each}
	{/if}

	{#if tags.length > 0}
		<div class="divider"></div>
		<button
			class="section-header"
			onclick={() => (tagsOpen = !tagsOpen)}
			aria-expanded={tagsOpen}
		>
			<span class="section-label">Tags</span>
			<span class="chevron" class:open={tagsOpen}><ChevronRight size={13} /></span>
		</button>
		{#if tagsOpen}
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
								onclick={() => libraryFiltersStore.toggleTag(tag)}
							>
								<X size={11} />
							</button>
						</div>
					{/each}
				</div>
			{/if}
			<div class="tag-list">
				{#each visibleTags.slice(0, 7) as [tag, count] (tag)}
					<button class="tag-row" onclick={() => libraryFiltersStore.toggleTag(tag)}>
						<span class="row-label">#{tag}</span>
						<span class="nav-count">{count}</span>
					</button>
				{/each}
				{#if visibleTags.length === 0}
					<div class="tag-empty">No tags match</div>
				{/if}
			</div>
			<a href="/tags" class="manage-tags-link">
				<span>Manage Tags</span>
				<span class="nav-count">{tags.length}</span>
			</a>
		{/if}
	{/if}
{/snippet}

<div class="app-root" class:mobile={isMobile}>
	{#if !isMobile}
		<div class="sidebar">
			<div class="brand-row">
				<div class="brand">
					<span class="brand-mark"><Logo size={17} /></span>
					Legere
				</div>
			</div>
			<button onclick={() => uiStore.openAddSource()} class="add-source-btn">
				<Plus size={15} />
				Add source
			</button>
			{@render sidebarNav()}

			<div class="sidebar-spacer"></div>
			{#if showUpdatePrompt && updateAvailable}
				<UpdateToast
					version={updateAvailable.version}
					notes={updateAvailable.notes}
					variant="inline"
					onUpdate={() => goto('/settings')}
					onDismiss={dismissUpdate}
				/>
			{/if}
		</div>
	{:else if !isReader}
		<div class="topbar">
			<span class="brand">
				<span class="brand-mark"><Logo size={16} /></span>
				Legere
			</span>
			<div class="topbar-actions">
				<button
					onclick={() => uiStore.openAddSource()}
					class="btn btn-icon btn-secondary"
					aria-label="Add source"
				>
					<Plus size={16} />
				</button>
			</div>
		</div>
	{/if}

	<div class="content" class:mobile={isMobile} class:mobile-reader={isMobile && isReader}>
		{@render children()}
	</div>

	{#if isMobile && !isReader}
		<div class="bottom-bar">
			{#each navItems as item (item.href)}
				<a href={item.href} class="bottom-nav-item" class:active={isActive(item.href)}>
					<item.Icon size={18} />
					{item.label}
				</a>
			{/each}
		</div>
	{/if}
</div>

{#if showUpdatePrompt && isMobile && updateAvailable}
	<UpdateToast
		version={updateAvailable.version}
		notes={updateAvailable.notes}
		onUpdate={() => goto('/settings')}
		onDismiss={dismissUpdate}
	/>
{/if}

<style>
	.app-root {
		display: flex;
		height: 100vh;
		background: var(--color-bg);
		color: var(--color-text);
	}
	.app-root.mobile {
		flex-direction: column;
	}

	.sidebar {
		display: flex;
		flex-direction: column;
		width: 256px;
		flex: none;
		padding: 24px 16px;
		height: 100%;
		border-right: 1px solid var(--color-divider-strong);
		overflow-y: auto;
	}
	.brand-row {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 9px;
		margin-bottom: 22px;
		padding: 0 6px;
	}
	.brand {
		display: flex;
		align-items: center;
		gap: 8px;
		font-family: var(--font-heading);
		font-weight: 700;
		font-size: 18px;
		letter-spacing: -0.015em;
	}
	.brand-mark {
		display: flex;
		color: var(--color-accent);
		flex: none;
	}
	.nav-list {
		display: flex;
		flex-direction: column;
		gap: 0;
	}
	.nav-item,
	.row-item {
		display: flex;
		align-items: center;
		gap: 10px;
		text-align: left;
		background: transparent;
		color: var(--color-text);
		border: none;
		border-radius: 10px;
		font-family: var(--font-body);
		font-weight: 400;
		font-size: 14px;
		padding: 9px 10px;
		cursor: pointer;
		text-decoration: none;
		width: 100%;
	}
	.row-item {
		gap: 9px;
		padding: 7px 10px;
	}
	.nav-item.active,
	.row-item.active {
		background: var(--color-surface);
		color: var(--color-accent);
	}
	.nav-item.active {
		font-weight: 600;
	}
	.row-item.active {
		font-weight: 600;
	}
	.nav-label,
	.row-label {
		flex: 1;
		font-size: 13.5px;
	}
	.nav-item .nav-label {
		font-size: 14px;
	}
	.nav-count {
		font-size: 11px;
		font-family: var(--font-body);
		color: var(--color-muted);
	}

	.dot {
		width: 7px;
		height: 7px;
		border-radius: 50%;
		flex: none;
	}

	.divider {
		height: 1px;
		background: var(--color-divider);
		margin: 18px 6px;
	}
	.section-label {
		font-size: 10.5px;
		letter-spacing: 0.06em;
		text-transform: uppercase;
		color: var(--color-muted);
		margin: 2px 0 8px 10px;
	}

	.section-header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		width: 100%;
		background: none;
		border: none;
		cursor: pointer;
		padding: 0 4px 0 10px;
		margin: 2px 0 8px 0;
	}
	.section-header .section-label {
		margin: 0;
	}
	.chevron {
		display: flex;
		color: var(--color-muted);
		transition: transform var(--duration-fast) var(--ease-snap);
	}
	.chevron.open {
		transform: rotate(90deg);
	}
	.tag-search {
		display: flex;
		align-items: center;
		gap: 7px;
		background: var(--color-surface);
		border-radius: 10px;
		padding: 7px 10px;
		margin: 0 6px 10px;
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
		max-height: none;
		overflow: visible;
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

	.sidebar-spacer {
		flex: 1;
		min-height: 16px;
	}
	.add-source-btn {
		display: flex;
		flex: none;
		align-items: center;
		justify-content: center;
		gap: 7px;
		background: var(--color-accent);
		color: var(--color-accent-fg);
		border: none;
		border-radius: 12px;
		font-family: var(--font-heading);
		font-weight: 600;
		font-size: 13.5px;
		padding: 12px;
		cursor: pointer;
		margin-bottom: 18px;
		transition: transform var(--duration-fast) var(--ease-snap), background var(--duration-base) var(--ease-snap);
	}
	.add-source-btn:hover {
		background: color-mix(in srgb, var(--color-accent) 88%, black);
	}
	.add-source-btn:active {
		transform: scale(0.97);
	}

	.topbar {
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: calc(12px + env(safe-area-inset-top)) calc(16px + env(safe-area-inset-right)) 12px
			calc(16px + env(safe-area-inset-left));
		position: sticky;
		top: 0;
		background: var(--color-bg);
		z-index: 5;
	}
	.topbar .brand {
		font-size: 16px;
	}
	.topbar-actions {
		display: flex;
		align-items: center;
		gap: 8px;
	}

	.content {
		flex: 1;
		min-width: 0;
		height: 100%;
		overflow-y: auto;
	}
	.content.mobile {
		padding-bottom: calc(70px + env(safe-area-inset-bottom));
	}
	.content.mobile-reader {
		padding-bottom: 0;
	}

	.bottom-bar {
		display: flex;
		align-items: stretch;
		border-top: 1px solid var(--color-divider-strong);
		background: var(--color-bg);
		flex: none;
		padding-bottom: env(safe-area-inset-bottom);
		padding-left: env(safe-area-inset-left);
		padding-right: env(safe-area-inset-right);
	}
	.bottom-nav-item {
		flex: 1;
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		gap: 2px;
		min-height: 44px;
		padding: 8px 0 10px;
		background: none;
		border: none;
		color: var(--color-text);
		font-size: 11px;
		font-family: var(--font-body);
		font-weight: 500;
		text-decoration: none;
	}
	.bottom-nav-item.active {
		color: var(--color-accent);
	}
</style>
