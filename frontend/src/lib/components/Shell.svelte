<script lang="ts">
	import { page } from '$app/state';
	import Library from '$lib/icons/Library.svelte';
	import Star from '$lib/icons/Star.svelte';
	import Highlighter from '$lib/icons/Highlighter.svelte';
	import SettingsIcon from '$lib/icons/Settings.svelte';
	import Plus from '$lib/icons/Plus.svelte';
	import Moon from '$lib/icons/Moon.svelte';
	import Sun from '$lib/icons/Sun.svelte';
	import { articlesStore } from '$lib/stores/articles.svelte';
	import { settingsStore } from '$lib/stores/settings.svelte';
	import { uiStore } from '$lib/stores/ui.svelte';
	import { libraryFiltersStore } from '$lib/stores/libraryFilters.svelte';
	import { sourceDotColor } from '$lib/sourceColor';
	import { deriveCategories, deriveTags } from '$lib/deriveCategories';
	let { children } = $props();

	let isMobile = $state(typeof window !== 'undefined' ? window.innerWidth < 768 : false);
	let isReader = $derived(page.url.pathname.startsWith('/reader/'));

	$effect(() => {
		function onResize() {
			isMobile = window.innerWidth < 768;
		}
		window.addEventListener('resize', onResize);
		return () => window.removeEventListener('resize', onResize);
	});

	const navItems = [
		{ href: '/', label: 'Library', Icon: Library, count: () => articlesStore.items.length },
		{ href: '/favorites', label: 'Favorites', Icon: Star, count: () => articlesStore.favoritedCount },
		{ href: '/highlights', label: 'Highlights', Icon: Highlighter, count: () => 0 },
		{ href: '/settings', label: 'Settings', Icon: SettingsIcon, count: null as (() => number) | null }
	];

	function isActive(href: string): boolean {
		return href === '/' ? page.url.pathname === '/' : page.url.pathname.startsWith(href);
	}

	function toggleTheme() {
		const next = settingsStore.current.app_theme === 'dark' ? 'light' : 'dark';
		settingsStore.update({ app_theme: next });
	}

	// Sidebar Categories/Tags: the design's mock data hardcodes a fixed
	// Design/Technology/Culture taxonomy with no real analogue in this
	// app. Categories here are derived from each article's real source
	// instead (same dot+label+count shape, same click-to-filter
	// behavior); tags are real, parsed from the source feed's
	// `<category>` elements at capture time.
	let categories = $derived(deriveCategories(articlesStore.items));
	let tags = $derived(deriveTags(articlesStore.items));
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
		{#each categories as [name, count] (name)}
			<button
				class="row-item"
				class:active={libraryFiltersStore.sourceName === name}
				onclick={() => libraryFiltersStore.toggleSource(name)}
			>
				<span class="dot" style:background={sourceDotColor(name)}></span>
				<span class="row-label">{name}</span>
				<span class="nav-count">{count}</span>
			</button>
		{/each}
	{/if}

	{#if tags.length > 0}
		<div class="divider"></div>
		<div class="section-label">Tags</div>
		<div class="tag-chips">
			{#each tags as [tag] (tag)}
				<button
					class="tag-chip"
					class:active={libraryFiltersStore.tags.includes(tag)}
					onclick={() => libraryFiltersStore.toggleTag(tag)}
				>
					#{tag}
				</button>
			{/each}
		</div>
	{/if}
{/snippet}

<div class="app-root" class:mobile={isMobile}>
	{#if !isMobile}
		<div class="sidebar">
			<div class="brand-row">
				<div class="brand">
					<span class="brand-dot"></span>
					Legere
				</div>
				<button
					class="btn btn-icon btn-secondary theme-toggle"
					onclick={toggleTheme}
					aria-label="Toggle theme"
				>
					{#if settingsStore.current.app_theme === 'light'}
						<Moon size={14} />
					{:else}
						<Sun size={14} />
					{/if}
				</button>
			</div>
			{@render sidebarNav()}

			<div class="sidebar-spacer"></div>
			<button onclick={() => uiStore.openAddSource()} class="add-source-btn">
				<Plus size={15} />
				Add source
			</button>
		</div>
	{:else if !isReader}
		<div class="topbar">
			<span class="brand">
				<span class="brand-dot"></span>
				Legere
			</span>
			<div class="topbar-actions">
				<button
					class="btn btn-icon btn-secondary"
					onclick={toggleTheme}
					aria-label="Toggle theme"
				>
					{#if settingsStore.current.app_theme === 'light'}
						<Moon size={15} />
					{:else}
						<Sun size={15} />
					{/if}
				</button>
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
		border-right: 1px solid var(--color-divider);
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
		gap: 9px;
		font-family: var(--font-heading);
		font-weight: 700;
		font-size: 19px;
		letter-spacing: -0.01em;
	}
	.brand-dot {
		width: 10px;
		height: 10px;
		border-radius: 50%;
		background: var(--color-accent);
		flex: none;
	}
	.theme-toggle {
		width: 28px;
		height: 28px;
		border-radius: 50%;
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

	.tag-chips {
		display: flex;
		flex-wrap: wrap;
		gap: 6px;
		padding: 0 6px;
	}
	.tag-chip {
		font-size: 11px;
		padding: 5px 11px;
		border-radius: 999px;
		background: var(--color-surface);
		color: var(--color-muted);
		border: none;
		cursor: pointer;
		font-family: var(--font-body);
	}
	.tag-chip.active {
		background: var(--color-accent);
		color: var(--color-accent-fg);
	}

	.sidebar-spacer {
		flex: 1;
		min-height: 16px;
	}
	.add-source-btn {
		display: flex;
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
		position: sticky;
		bottom: 0;
		transition: transform 0.08s ease;
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
		border-top: 1px solid var(--color-divider);
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
