<script lang="ts">
	import { onMount } from 'svelte';
	import { page } from '$app/state';
	import Logo from '$lib/icons/Logo.svelte';
	import Library from '$lib/icons/Library.svelte';
	import Star from '$lib/icons/Star.svelte';
	import SettingsIcon from '$lib/icons/Settings.svelte';
	import Rss from '$lib/icons/Rss.svelte';
	import Plus from '$lib/icons/Plus.svelte';
	import ChevronRight from '$lib/icons/ChevronRight.svelte';
	import TagBrowser from '$lib/components/TagBrowser.svelte';
	import { libraryStatsStore } from '$lib/stores/libraryStats.svelte';
	import { captureJobsStore } from '$lib/stores/captureJobs.svelte';
	import { uiStore } from '$lib/stores/ui.svelte';
	import { sidebarStore } from '$lib/stores/sidebar.svelte';
	import { libraryFiltersStore } from '$lib/stores/libraryFilters.svelte';
	import PanelLeftClose from '$lib/icons/lucide/panel-left-close.svelte';
	import PanelLeftOpen from '$lib/icons/lucide/panel-left-open.svelte';
	import CircleArrowUp from '$lib/icons/lucide/circle-arrow-up.svelte';
	import Hash from '$lib/icons/Hash.svelte';
	import Folder from '$lib/icons/Folder.svelte';
	import Refresh from '$lib/icons/Refresh.svelte';
	import CategoryIcon from '$lib/components/CategoryIcon.svelte';
	import { goto } from '$app/navigation';
	import { isTauri } from '$lib/platform';
	import { checkForUpdate, getLastDismissedVersion, setLastDismissedVersion } from '$lib/update';
	import UpdateToast from '$lib/components/UpdateToast.svelte';

	let { children } = $props();

	let isMobile = $state(typeof window !== 'undefined' ? window.innerWidth < 768 : false);
	let isReader = $derived(page.url.pathname.startsWith('/reader/'));
	let isSettings = $derived(page.url.pathname === '/settings');
	// The FAB is a "jump into the library and add a source" affordance, so it
	// only makes sense on the Library page itself. Categories, tags, and
	// favorites are filtered *views* of the library, not places to add a
	// source from, and Sources already has its own prominent "Add source"
	// button in its own header — a second identical affordance there would be
	// redundant. Settings isn't about adding anything at all either.
	let isLibrary = $derived(page.url.pathname === '/');
	// The mobile activity dock (a thin bar docked above `.bottom-bar`, see
	// its own styles below) — hidden on the reader like the rest of the
	// chrome, but *not* gated to the Library page the way the FAB is: this
	// isn't a second "add" affordance, it's a status readout, so it stays useful
	// on the one page that's actually about sources too.
	let showActivityDock = $derived(
		isMobile && !isReader && captureJobsStore.jobs.length > 0
	);

	$effect(() => {
		function onResize() {
			isMobile = window.innerWidth < 768;
		}
		window.addEventListener('resize', onResize);
		return () => window.removeEventListener('resize', onResize);
	});

	// ── Background update check ──────────────────────────────────────────────
	let updateAvailable = $state<{ version: string; notes?: string | null } | null>(null);

	/** Fire-and-forget: check for an update without blocking or delaying page load. */
	async function backgroundCheckForUpdate() {
		if (!isTauri()) return;
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

	// Runs exactly once per app load, not on every navigation: deliberately
	// an `onMount`-shaped one-shot, not an `$effect` that happens to read no
	// reactive state (a previous version read `isSettings` here, which made
	// this re-fire — and re-hit the network — on every single navigation
	// into or out of Settings).
	onMount(() => {
		backgroundCheckForUpdate();
	});

	// Both the desktop sidebar and the mobile bottom bar carry the same
	// four top-level destinations — Library, Favorites, Sources, Settings.
	const navItems = [
		{ href: '/', label: 'Library', Icon: Library, count: () => libraryStatsStore.totalCount },
		{ href: '/favorites', label: 'Favorites', Icon: Star, count: () => libraryStatsStore.favoritedCount },
		{ href: '/sources', label: 'Sources', Icon: Rss, count: null as (() => number) | null },
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

	// The virtual "Uncategorized" entry (see `libraryStatsStore`) always
	// stays pinned at the top of the sidebar list — it's the default
	// destination, not just another folder — while the real, user-created
	// categories below it are capped to the busiest 5 (by article count) so
	// a library with dozens of categories doesn't push Tags and the rest of
	// the sidebar out of view. The full set is always one click away via
	// "Manage categories" (`/categories`).
	let uncategorized = $derived(categories.find((c) => c.id === '__uncategorized__') ?? null);
	const topCategoryCount = 5;
	let topCategories = $derived(
		categories
			.filter((c) => c.id !== '__uncategorized__')
			.slice()
			.sort((a, b) => b.article_count - a.article_count)
			.slice(0, topCategoryCount)
	);

	// Categories/Tags sections: both collapsible (state survives navigation
	// — this component never remounts between routes). Search/browse/facet-
	// narrow logic for tags lives in the shared `TagBrowser` (also used by
	// the mobile Tags sheet); this component only owns whether each section
	// is expanded.
	let categoriesOpen = $state(true);
	let tagsOpen = $state(true);

	// ── Collapsed rail ───────────────────────────────────────────────────────
	// The sidebar can collapse to a 64px icon rail (`sidebarStore`, persisted
	// per device). Sections that are lists in the full sidebar (Categories,
	// Tags, the update card) become one icon each that opens a flyout beside
	// the rail; only one flyout is open at a time.
	let collapsed = $derived(sidebarStore.collapsed);
	type Flyout = 'categories' | 'tags' | 'update';
	let openFlyout = $state<Flyout | null>(null);
	const modKey =
		typeof navigator !== 'undefined' && /Mac/.test(navigator.platform) ? '\u2318' : 'Ctrl+';

	function toggleFlyout(flyout: Flyout) {
		openFlyout = openFlyout === flyout ? null : flyout;
	}

	function toggleSidebar() {
		openFlyout = null;
		sidebarStore.toggle();
	}

	let onCategoryRoute = $derived(page.url.pathname.startsWith('/category/'));
	let activeTagCount = $derived(libraryFiltersStore.tags.length);

	// Navigating (e.g. picking a category in the flyout) closes it.
	$effect(() => {
		void page.url.pathname;
		openFlyout = null;
	});

	$effect(() => {
		function onPointerDown(event: PointerEvent) {
			if (openFlyout && !(event.target as Element | null)?.closest?.('.rail-slot')) {
				openFlyout = null;
			}
		}
		function onKeydown(event: KeyboardEvent) {
			if (event.key === 'Escape' && openFlyout) {
				document.querySelector<HTMLElement>('.rail-slot.has-flyout > .rail-item')?.focus();
				openFlyout = null;
				return;
			}
			if (
				(event.ctrlKey || event.metaKey) &&
				!event.altKey &&
				!event.shiftKey &&
				event.key.toLowerCase() === 'b' &&
				!isMobile
			) {
				const target = event.target as HTMLElement | null;
				if (target && (target.isContentEditable || /^(INPUT|TEXTAREA|SELECT)$/.test(target.tagName))) {
					return;
				}
				event.preventDefault();
				toggleSidebar();
			}
		}
		window.addEventListener('pointerdown', onPointerDown);
		window.addEventListener('keydown', onKeydown);
		return () => {
			window.removeEventListener('pointerdown', onPointerDown);
			window.removeEventListener('keydown', onKeydown);
		};
	});
</script>

{#snippet categoryRows()}
	{#if uncategorized}
		<a
			href="/category/{uncategorized.id}"
			class="row-item"
			class:active={isActiveCategory(uncategorized.id)}
		>
			<CategoryIcon icon={uncategorized.icon} size={13} />
			<span class="row-label">{uncategorized.name}</span>
			<span class="nav-count">{uncategorized.article_count}</span>
		</a>
	{/if}
	{#each topCategories as category (category.id)}
		<a
			href="/category/{category.id}"
			class="row-item"
			class:active={isActiveCategory(category.id)}
		>
			<CategoryIcon icon={category.icon} size={13} />
			<span class="row-label">{category.name}</span>
			<span class="nav-count">{category.article_count}</span>
		</a>
	{/each}
	<a href="/categories" class="manage-link">
		<span>Manage categories</span>
		<span class="nav-count">{categories.length - (uncategorized ? 1 : 0)}</span>
	</a>
{/snippet}

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
		<button
			class="section-header"
			onclick={() => (categoriesOpen = !categoriesOpen)}
			aria-expanded={categoriesOpen}
		>
			<span class="section-label">Categories</span>
			<span class="chevron" class:open={categoriesOpen}><ChevronRight size={13} /></span>
		</button>
		{#if categoriesOpen}
			{@render categoryRows()}
		{/if}
	{/if}

	{#if tags.length > 0}
		<button
			class="section-header"
			onclick={() => (tagsOpen = !tagsOpen)}
			aria-expanded={tagsOpen}
		>
			<span class="section-label">Tags</span>
			<span class="chevron" class:open={tagsOpen}><ChevronRight size={13} /></span>
		</button>
		{#if tagsOpen}
			<TagBrowser />
		{/if}
	{/if}
{/snippet}

<div class="app-root" class:mobile={isMobile}>
	{#if !isMobile}
		<div class="sidebar" class:collapsed>
			{#if collapsed}
				<div class="rail">
					<span class="brand-mark rail-mark"><Logo size={24} /></span>
					<button
						class="rail-item"
						data-tip="Expand sidebar  {modKey}B"
						aria-label="Expand sidebar"
						aria-expanded="false"
						onclick={toggleSidebar}
					>
						<PanelLeftOpen size={18} />
					</button>
					<button
						class="btn btn-primary btn-icon rail-add"
						data-tip="Add source"
						aria-label="Add source"
						onclick={() => uiStore.openAddSource()}
					>
						<Plus size={18} />
					</button>
					{#if captureJobsStore.jobs.length > 0}
						{@const activityLabel =
							captureJobsStore.failedCount > 0
								? `${captureJobsStore.failedCount} failed to add`
								: 'Adding\u2026'}
						<button
							class="rail-item rail-activity"
							class:rail-activity-failed={captureJobsStore.failedCount > 0}
							data-tip={activityLabel}
							aria-label={activityLabel}
							onclick={() => uiStore.openCaptureJobs()}
						>
							<Refresh size={16} spinning={captureJobsStore.runningCount > 0} />
							{#if captureJobsStore.failedCount > 0}<span class="rail-dot rail-dot-danger"></span>{/if}
						</button>
					{/if}

					<nav class="rail-group" aria-label="Main">
						{#each navItems as item (item.href)}
							{@const tip = item.count ? `${item.label} \u00b7 ${item.count()}` : item.label}
							<a
								href={item.href}
								class="rail-item"
								class:active={isActive(item.href)}
								data-tip={tip}
								aria-label={tip}
							>
								<item.Icon size={18} />
							</a>
						{/each}
					</nav>

					{#if categories.length > 0 || tags.length > 0}<div class="rail-divider"></div>{/if}

					{#if categories.length > 0}
						<div class="rail-slot" class:has-flyout={openFlyout === 'categories'}>
							<button
								class="rail-item"
								class:active={onCategoryRoute}
								class:open={openFlyout === 'categories'}
								data-tip="Categories"
								aria-label="Categories"
								aria-haspopup="true"
								aria-expanded={openFlyout === 'categories'}
								onclick={() => toggleFlyout('categories')}
							>
								<Folder size={18} />
							</button>
							{#if openFlyout === 'categories'}
								<div class="flyout" role="group" aria-label="Categories">
									<div class="flyout-title section-label">Categories</div>
									{@render categoryRows()}
								</div>
							{/if}
						</div>
					{/if}

					{#if tags.length > 0}
						<div class="rail-slot" class:has-flyout={openFlyout === 'tags'}>
							<button
								class="rail-item"
								class:active={activeTagCount > 0}
								class:open={openFlyout === 'tags'}
								data-tip={activeTagCount > 0 ? `Tags \u00b7 ${activeTagCount} selected` : 'Tags'}
								aria-label={activeTagCount > 0 ? `Tags, ${activeTagCount} selected` : 'Tags'}
								aria-haspopup="true"
								aria-expanded={openFlyout === 'tags'}
								onclick={() => toggleFlyout('tags')}
							>
								<Hash size={18} />
								{#if activeTagCount > 0}<span class="rail-badge">{activeTagCount}</span>{/if}
							</button>
							{#if openFlyout === 'tags'}
								<div class="flyout" role="group" aria-label="Tags">
									<div class="flyout-title section-label">Tags</div>
									<TagBrowser maxVisible={12} />
								</div>
							{/if}
						</div>
					{/if}

					<div class="sidebar-spacer"></div>

					{#if showUpdatePrompt && updateAvailable}
						<div class="rail-slot" class:has-flyout={openFlyout === 'update'}>
							<button
								class="rail-item"
								class:open={openFlyout === 'update'}
								data-tip="Update available \u2014 v{updateAvailable.version}"
								aria-label="Update available, version {updateAvailable.version}"
								aria-haspopup="true"
								aria-expanded={openFlyout === 'update'}
								onclick={() => toggleFlyout('update')}
							>
								<CircleArrowUp size={18} />
								<span class="rail-dot"></span>
							</button>
							{#if openFlyout === 'update'}
								<div class="flyout flyout-flush flyout-bottom" role="group" aria-label="Update available">
									<UpdateToast
										version={updateAvailable.version}
										notes={updateAvailable.notes}
										variant="inline"
										onUpdate={() => goto('/settings')}
										onDismiss={dismissUpdate}
									/>
								</div>
							{/if}
						</div>
					{/if}
				</div>
			{:else}
				<div class="brand-row">
					<div class="brand">
						<span class="brand-mark"><Logo size={24} /></span>
						Legere
					</div>
					<button
						class="icon-toggle"
						title="Collapse sidebar ({modKey}B)"
						aria-label="Collapse sidebar"
						aria-expanded="true"
						onclick={toggleSidebar}
					>
						<PanelLeftClose size={16} />
					</button>
				</div>
				<button onclick={() => uiStore.openAddSource()} class="btn btn-primary btn-block add-source-btn">
					<Plus size={15} />
					Add source
				</button>
				{#if captureJobsStore.jobs.length > 0}
					<button
						class="activity-btn"
						class:activity-btn-failed={captureJobsStore.failedCount > 0}
						onclick={() => uiStore.openCaptureJobs()}
					>
						{#if captureJobsStore.runningCount > 0}
							<Refresh size={13} spinning />
						{/if}
						<span class="row-label">
							{captureJobsStore.failedCount > 0
								? `${captureJobsStore.failedCount} failed to add`
								: 'Adding\u2026'}
						</span>
					</button>
				{/if}
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
			{/if}
		</div>
	{/if}

	<div class="content" class:mobile={isMobile} class:mobile-reader={isMobile && isReader}>
		{@render children()}
	</div>

	{#if isMobile && isLibrary}
		<button
			class="fab"
			class:fab-raised={showActivityDock}
			onclick={() => uiStore.openAddSource()}
			aria-label="Add source"
		>
			<Plus size={22} />
		</button>
	{/if}
	{#if showActivityDock}
		<button
			class="activity-dock"
			class:activity-dock-failed={captureJobsStore.failedCount > 0}
			onclick={() => uiStore.openCaptureJobs()}
		>
			{#if captureJobsStore.runningCount > 0}
				<Refresh size={12} spinning />
			{/if}
			<span class="row-label">
				{captureJobsStore.failedCount > 0
					? `${captureJobsStore.failedCount} failed to add`
					: `Adding ${captureJobsStore.jobs.length}\u2026`}
			</span>
			<ChevronRight size={12} />
		</button>
	{/if}
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
		/* No visible scrollbar: the reserved track nudged every row left the
		   moment the section list (usually a big Tags browser) overflowed.
		   Wheel and keyboard scrolling still work — the handle is just hidden,
		   since this column rarely overflows and never needs the affordance. */
		scrollbar-width: none;
	}
	.sidebar::-webkit-scrollbar {
		display: none;
		width: 0;
		height: 0;
	}
	.sidebar > :global(*) {
		animation: sidebar-fade var(--duration-base) var(--ease-snap);
	}
	@keyframes sidebar-fade {
		from {
			opacity: 0;
		}
	}
	.icon-toggle {
		display: flex;
		align-items: center;
		justify-content: center;
		width: 28px;
		height: 28px;
		flex: none;
		background: transparent;
		border: none;
		border-radius: 8px;
		color: var(--color-muted);
		cursor: pointer;
		transition:
			background var(--duration-fast) var(--ease-snap),
			color var(--duration-fast) var(--ease-snap);
	}
	.icon-toggle:hover {
		background: var(--color-surface);
		color: var(--color-text);
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
		gap: 10px;
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

	.section-label {
		font-size: 10.5px;
		letter-spacing: 0.06em;
		text-transform: uppercase;
		color: var(--color-muted);
	}
	/* Section rhythm without rules — the old hairline dividers between the
	   nav list, Categories, and Tags are gone; spacing alone separates the
	   sections now (more above a header than below it). */
	.section-header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		width: 100%;
		background: none;
		border: none;
		cursor: pointer;
		padding: 0 4px 0 10px;
		margin: 16px 0 8px 0;
	}
	.chevron {
		display: flex;
		color: var(--color-muted);
		transition: transform var(--duration-fast) var(--ease-snap);
	}
	.chevron.open {
		transform: rotate(90deg);
	}
	/* Tags section rows now render via the shared `TagBrowser` component;
	   only the collapsible section header (above) still styles here. */

	/* Mirrors `TagBrowser`'s own `.manage-tags-link` — same idiom for the
	   Categories section's "see the rest" link, once the list above it is
	   capped to the busiest 5. */
	.manage-link {
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
	.manage-link:hover {
		background: var(--color-surface);
		color: var(--color-accent);
	}

	.sidebar-spacer {
		flex: 1;
		min-height: 16px;
	}
	/* Same shared `.btn .btn-primary` chrome as the Sources page's own
	   "Add source" button (`routes/sources/+page.svelte`) — only the layout
	   (full sidebar width, spacing below) is bespoke here. Don't reintroduce
	   one-off padding/radius/font overrides; this is the same action
	   rendered in a second place; it should always look identical. */
	.add-source-btn {
		margin-bottom: 18px;
	}
	/* Only rendered while `captureJobsStore` has something to show (see the
	   markup above) — a background "add a source" job in progress, or one
	   that failed and is waiting on a retry/dismiss via `CaptureJobsPanel`. */
	.activity-btn {
		display: flex;
		align-items: center;
		gap: 8px;
		width: 100%;
		background: var(--color-surface);
		color: var(--color-muted);
		border: none;
		border-radius: 10px;
		font-family: var(--font-body);
		font-size: 12.5px;
		padding: 9px 10px;
		cursor: pointer;
		margin-bottom: 10px;
		text-align: left;
	}
	.activity-btn-failed {
		color: var(--color-danger);
	}
	.activity-btn :global(svg) {
		flex: none;
	}

	/* ── Collapsed rail ─────────────────────────────────────────────────────
	   Icons only; flyouts and tooltips extend past the rail's edge, so it
	   stops clipping (it never needs to scroll at this density) and sits
	   above the content column. */
	.sidebar.collapsed {
		width: 64px;
		padding: 20px 12px;
		overflow: visible;
		position: relative;
		z-index: 30;
	}
	.rail {
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 4px;
		flex: 1;
		min-height: 0;
	}
	.rail-mark {
		margin: 2px 0 10px;
	}
	.rail-group {
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 2px;
		margin-top: 10px;
	}
	.rail-divider {
		width: 24px;
		height: 1px;
		background: var(--color-divider);
		margin: 8px 0;
	}
	.rail-slot {
		position: relative;
		display: flex;
	}
	.rail-item {
		position: relative;
		display: flex;
		align-items: center;
		justify-content: center;
		width: 40px;
		height: 40px;
		flex: none;
		background: transparent;
		color: var(--color-text);
		border: none;
		border-radius: 10px;
		cursor: pointer;
		text-decoration: none;
		transition:
			background var(--duration-fast) var(--ease-snap),
			color var(--duration-fast) var(--ease-snap),
			transform var(--duration-fast) var(--ease-snap);
	}
	.rail-item:hover,
	.rail-item.open {
		background: var(--color-surface);
	}
	.rail-item:active {
		transform: scale(0.96);
	}
	.rail-item.active {
		background: var(--color-surface);
		color: var(--color-accent);
	}
	/* "Add source" keeps the shared .btn-primary chrome; only its box is
	   widened to the rail's 40px target (layout-only, see DESIGN.md). */
	.rail-add {
		width: 40px;
		height: 40px;
		margin-top: 2px;
	}
	.rail-activity {
		color: var(--color-muted);
	}
	.rail-activity-failed {
		color: var(--color-danger);
	}
	.rail-badge {
		position: absolute;
		top: 3px;
		right: 2px;
		min-width: 15px;
		height: 15px;
		padding: 0 4px;
		border-radius: 999px;
		background: var(--color-accent);
		color: var(--color-accent-fg);
		font-size: 10px;
		font-weight: 600;
		line-height: 15px;
		text-align: center;
	}
	.rail-dot {
		position: absolute;
		top: 8px;
		right: 8px;
		width: 7px;
		height: 7px;
		border-radius: 50%;
		background: var(--color-accent);
		box-shadow: 0 0 0 2px var(--color-bg);
	}
	.rail-dot-danger {
		background: var(--color-danger);
	}

	/* Tooltips: native `title` takes ~1s and is unstyled in WebKitGTK, so
	   the rail draws its own — delayed on hover (so sweeping the mouse down
	   the rail doesn't strobe), immediate on keyboard focus. */
	.rail [data-tip]::after {
		content: attr(data-tip);
		position: absolute;
		left: calc(100% + 12px);
		top: 50%;
		transform: translateY(-50%);
		padding: 5px 10px;
		border-radius: var(--radius-md);
		background: var(--color-surface-raised);
		color: var(--color-text);
		box-shadow: var(--shadow-md);
		font-family: var(--font-body);
		font-size: 12px;
		font-weight: 500;
		white-space: nowrap;
		pointer-events: none;
		opacity: 0;
		z-index: 50;
	}
	.rail [data-tip]:not(.open):hover::after {
		opacity: 1;
		transition: opacity var(--duration-fast) var(--ease-snap) 0.35s;
	}
	.rail [data-tip]:not(.open):focus-visible::after {
		opacity: 1;
		transition: opacity var(--duration-fast) var(--ease-snap);
	}

	.flyout {
		position: absolute;
		left: calc(100% + 16px);
		top: -4px;
		width: 272px;
		max-height: min(72vh, 540px);
		overflow-y: auto;
		padding: 12px;
		background: var(--color-surface-raised);
		border-radius: var(--radius-lg);
		box-shadow: var(--shadow-md);
		z-index: 40;
		animation: flyout-in var(--duration-base) var(--ease-snap);
	}
	.flyout-bottom {
		top: auto;
		bottom: -4px;
	}
	/* The update card is already a surface of its own — it *is* the flyout
	   here, so no card nested in a card. */
	.flyout-flush {
		padding: 0;
		background: var(--color-surface);
	}
	.flyout-flush :global(.update-toast) {
		margin-top: 0;
	}
	.flyout-title {
		padding: 0 4px 8px;
	}
	@keyframes flyout-in {
		from {
			opacity: 0;
			transform: translateX(-6px);
		}
	}

	.content {
		flex: 1;
		min-width: 0;
		height: 100%;
		overflow-y: auto;
	}
	/* No `.content.mobile` bottom padding here — `.bottom-bar` below is a
	   real flex sibling (not an overlay), so it already reserves its own
	   space in `.app-root`'s column layout; adding padding here on top of
	   that double-counted it, leaving a band of bare `--color-bg` between
	   the actual scrollable content and the bar (see the pull-to-refresh/
	   FAB comments in `ArticleCollection` for the bottom clearance that
	   *is* needed, which belongs to each page's own scroll container). */

	/* Floating add-source shortcut — replaces the removed mobile top bar's
	   `+` button now that there's no top bar to hold it. Anchored to the
	   viewport (not `.content`) so it stays put regardless of which
	   descendant actually scrolls; sits just above `.bottom-bar`, whose own
	   height (nav item ~62px + its own safe-area padding) this offset
	   mirrors so the two never overlap. Every scrollable page adds matching
	   bottom clearance so the FAB never covers the last row. `.fab-raised`
	   (applied while `.activity-dock` below is showing) adds exactly that
	   dock's own fixed height, so the FAB clears it the same way. */
	.fab {
		position: fixed;
		right: calc(20px + env(safe-area-inset-right));
		bottom: calc(78px + env(safe-area-inset-bottom));
		width: 56px;
		height: 56px;
		border-radius: 50%;
		display: flex;
		align-items: center;
		justify-content: center;
		background: var(--color-accent);
		color: var(--color-accent-fg);
		border: none;
		box-shadow: var(--shadow-lg);
		cursor: pointer;
		z-index: 10;
		transition: transform var(--duration-fast) var(--ease-snap), bottom var(--duration-base) var(--ease-snap);
	}
	.fab:active {
		transform: scale(0.94);
	}
	.fab-raised {
		bottom: calc(78px + 30px + env(safe-area-inset-bottom));
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

	/* A second, thin bar docked directly above `.bottom-bar` — a real flex
	   sibling (like `.bottom-bar` itself), not an overlay, so it never
	   floats over content or collides with the FAB (which raises itself by
	   exactly this bar's height via `.fab-raised` above). Deliberately kept
	   to one compact row (fixed `height`, no wrapping) rather than growing
	   with content — the point is a persistent, low-footprint status
	   readout, not a second header. An earlier version tried a floating
	   chip stacked above the FAB, then a full-width banner at the *top* of
	   the screen; both read as disconnected from the rest of the chrome. A
	   slim dock above the tab bar (the same place a music-player mini-bar
	   sits above a tab bar in other apps) is the familiar spot for "a
	   background operation is in progress, tap for details". */
	.activity-dock {
		display: flex;
		align-items: center;
		gap: 6px;
		flex: none;
		width: 100%;
		height: 30px;
		background: var(--color-surface-raised);
		color: var(--color-muted);
		border: none;
		border-top: 1px solid var(--color-divider);
		padding: 0 calc(14px + env(safe-area-inset-right)) 0 calc(14px + env(safe-area-inset-left));
		font-family: var(--font-body);
		font-size: 11.5px;
		font-weight: 500;
		text-align: left;
		cursor: pointer;
	}
	.activity-dock .row-label {
		flex: 1;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.activity-dock :global(svg) {
		flex: none;
	}
	.activity-dock-failed {
		color: var(--color-danger);
		background: color-mix(in srgb, var(--color-danger) 10%, var(--color-surface-raised));
	}
</style>
