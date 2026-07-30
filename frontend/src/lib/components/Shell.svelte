<script lang="ts">
	import { page } from '$app/state';
	import Library from '$lib/icons/Library.svelte';
	import Rss from '$lib/icons/Rss.svelte';
	import SettingsIcon from '$lib/icons/Settings.svelte';
	import Plus from '$lib/icons/Plus.svelte';
	import { articlesStore } from '$lib/stores/articles.svelte';
	import { uiStore } from '$lib/stores/ui.svelte';

	let { children } = $props();

	let isMobile = $state(typeof window !== 'undefined' ? window.innerWidth < 768 : false);

	$effect(() => {
		function onResize() {
			isMobile = window.innerWidth < 768;
		}
		window.addEventListener('resize', onResize);
		return () => window.removeEventListener('resize', onResize);
	});

	const navItems = [
		{ href: '/', label: 'Library', Icon: Library },
		{ href: '/sources', label: 'Sources', Icon: Rss },
		{ href: '/settings', label: 'Settings', Icon: SettingsIcon }
	];

	function isActive(href: string): boolean {
		return href === '/' ? page.url.pathname === '/' : page.url.pathname.startsWith(href);
	}
</script>

<div class="app-root" class:mobile={isMobile}>
	{#if !isMobile}
		<div class="sidebar">
			<div class="brand">Legere</div>

			<button onclick={() => uiStore.openAddSource()} class="btn btn-primary btn-block">
				<Plus />
				Add source
			</button>

			<nav class="nav-list">
				{#each navItems as item (item.href)}
					<a href={item.href} class="nav-item" class:active={isActive(item.href)}>
						<item.Icon />
						{item.label}
						{#if item.href === '/' && articlesStore.unreadCount > 0}
							<span class="unread-badge">{articlesStore.unreadCount}</span>
						{/if}
					</a>
				{/each}
			</nav>
		</div>
	{:else}
		<div class="topbar">
			<span class="brand">Legere</span>
			<button
				onclick={() => uiStore.openAddSource()}
				class="btn btn-icon btn-secondary"
				aria-label="Add source"
			>
				<Plus size={16} />
			</button>
		</div>
	{/if}

	<div class="content" class:mobile={isMobile}>
		{@render children()}
	</div>

	{#if isMobile}
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
		width: 220px;
		flex: none;
		padding: 20px 14px;
		gap: 20px;
		height: 100%;
	}
	.brand {
		font-family: var(--font-heading);
		font-weight: 600;
		font-size: 19px;
		letter-spacing: -0.01em;
		padding: 0 6px;
	}

	.nav-list {
		display: flex;
		flex-direction: column;
		gap: 2px;
	}
	.nav-item {
		display: flex;
		align-items: center;
		gap: 10px;
		text-align: left;
		background: transparent;
		color: var(--color-text);
		border: none;
		border-radius: var(--radius-md);
		font-family: var(--font-heading);
		font-weight: 500;
		font-size: 14px;
		padding: 8px 10px;
		cursor: pointer;
		text-decoration: none;
	}
	.nav-item.active {
		background: var(--color-surface);
		color: var(--color-accent);
	}
	.unread-badge {
		margin-left: auto;
		font-size: 11px;
		font-family: var(--font-body);
		font-weight: 600;
		color: var(--color-accent-200);
		background: var(--color-accent-800);
		padding: 1px 7px;
		border-radius: var(--radius-sm);
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
		font-size: 17px;
		padding: 0;
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
		font-family: var(--font-heading);
		font-weight: 500;
		text-decoration: none;
	}
	.bottom-nav-item.active {
		color: var(--color-accent);
	}
</style>
