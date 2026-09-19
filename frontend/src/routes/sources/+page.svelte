<script lang="ts">
	import { sourcesStore } from '$lib/stores/sources.svelte';
	import { uiStore } from '$lib/stores/ui.svelte';
	import Plus from '$lib/icons/Plus.svelte';
	import Rss from '$lib/icons/Rss.svelte';
	import LinkIcon from '$lib/icons/LinkIcon.svelte';
	import Refresh from '$lib/icons/Refresh.svelte';
	import Search from '$lib/icons/Search.svelte';
	import MoreVertical from '$lib/icons/MoreVertical.svelte';
	import Trash from '$lib/icons/Trash.svelte';
	import ExportResultCard from '$lib/components/ExportResultCard.svelte';
	import { formatRelativeTime } from '$lib/format';
	import type { ExportResult, Source } from '$lib/types';
	import * as api from '$lib/api';

	$effect(() => {
		sourcesStore.refresh();
	});

	let syncingIds = $state(new Set<string>());
	let syncingAll = $state(false);

	const typeLabelMap: Record<Source['source_type'], string> = {
		rss: 'RSS feed',
		direct: 'Direct link'
	};

	const statusDotColor: Record<Source['status'], string> = {
		active: 'var(--color-accent)',
		paused: 'var(--color-muted)',
		error: 'var(--color-danger)'
	};

	const statusLabel: Record<Source['status'], string> = {
		active: 'Active',
		paused: 'Paused',
		error: 'Error'
	};

	let search = $state('');
	const filteredSources = $derived.by(() => {
		const q = search.trim().toLowerCase();
		if (!q) return sourcesStore.items;
		return sourcesStore.items.filter((s) => s.name.toLowerCase().includes(q));
	});

	// Paused sources are excluded from both manual "sync all" and autosync
	// (see `sync_all_sources` in the backend), so grouping them apart from
	// active/error sources here mirrors that split for the user.
	const activeSources = $derived(filteredSources.filter((s) => s.status !== 'paused'));
	const pausedSources = $derived(filteredSources.filter((s) => s.status === 'paused'));

	async function syncAll() {
		syncingAll = true;
		try {
			await sourcesStore.syncAll();
		} finally {
			syncingAll = false;
		}
	}

	async function syncNow(id: string) {
		openMenuId = null;
		syncingIds = new Set(syncingIds).add(id);
		try {
			await sourcesStore.syncOne(id);
		} finally {
			const next = new Set(syncingIds);
			next.delete(id);
			syncingIds = next;
		}
	}

	function togglePause(id: string) {
		openMenuId = null;
		sourcesStore.togglePause(id);
	}

	async function removeSource(source: Source) {
		openMenuId = null;
		if (!confirm(`Remove "${source.name}"? Articles already saved from it are kept.`)) return;
		await sourcesStore.remove(source.id);
	}

	let exportResult = $state<ExportResult | null>(null);
	let exporting = $state(false);
	let exportError = $state('');

	async function exportSources() {
		exporting = true;
		exportError = '';
		menuOpen = false;
		try {
			exportResult = await api.exportSourcesCsv();
		} catch (e) {
			exportError = api.errorMessage(e);
		} finally {
			exporting = false;
		}
	}

	let menuOpen = $state(false);
	let menuEl = $state<HTMLElement | null>(null);

	// Which source card's overflow menu is open, if any — only one at a
	// time, scoped by source id rather than a bound element since the menus
	// live inside a `#each` block.
	let openMenuId = $state<string | null>(null);

	function handleClickOutside(e: MouseEvent) {
		const target = e.target as Node;
		if (menuOpen && menuEl && !menuEl.contains(target)) {
			menuOpen = false;
		}
		if (openMenuId) {
			const openEl = document.querySelector(`.source-menu[data-id="${CSS.escape(openMenuId)}"]`);
			if (!openEl || !openEl.contains(target)) {
				openMenuId = null;
			}
		}
	}
</script>

<svelte:window
	onclick={handleClickOutside}
	onkeydown={(e) => {
		if (e.key !== 'Escape') return;
		if (menuOpen) menuOpen = false;
		if (openMenuId) openMenuId = null;
	}}
/>

<div class="sources-page">
	<div class="header-row">
		<h1>Sources</h1>
		<div class="header-actions">
			<button class="btn btn-primary" onclick={() => uiStore.openAddSource()}>
				<Plus size={14} />
				Add source
			</button>
			<button
				class="btn btn-secondary sync-all-btn"
				onclick={syncAll}
				disabled={syncingAll || sourcesStore.items.length === 0}
				title="Sync all"
				aria-label="Sync all sources"
			>
				<Refresh size={14} spinning={syncingAll} />
				<span class="sync-all-label">Sync all</span>
			</button>
			<!-- Desktop has room to show these two actions outright, so the whole
			     overflow menu below is hidden there (`.header-overflow-only`) —
			     narrow/mobile viewports fall back to the ⋮ menu instead. -->
			<button class="btn btn-secondary header-desktop-only" onclick={() => uiStore.openImportSourcesDialog()}>
				Import CSV
			</button>
			<button class="btn btn-secondary header-desktop-only" disabled={exporting} onclick={exportSources}>
				{exporting ? 'Exporting…' : 'Export CSV'}
			</button>
			<div class="overflow-menu header-overflow-only" bind:this={menuEl}>
				<button
					class="btn btn-icon btn-secondary"
					onclick={() => (menuOpen = !menuOpen)}
					aria-label="More options"
					aria-expanded={menuOpen}
				>
					<MoreVertical />
				</button>
				{#if menuOpen}
					<div class="overflow-popover elev-md" role="menu">
						<button
							class="overflow-item"
							role="menuitem"
							onclick={() => {
								menuOpen = false;
								uiStore.openImportSourcesDialog();
							}}
						>
							Import CSV
						</button>
						<button class="overflow-item" role="menuitem" disabled={exporting} onclick={exportSources}>
							{exporting ? 'Exporting…' : 'Export CSV'}
						</button>
					</div>
				{/if}
			</div>
		</div>
	</div>

	{#if sourcesStore.items.length > 0}
		<div class="search-box">
			<Search size={15} />
			<input
				type="text"
				placeholder="Search sources"
				bind:value={search}
				spellcheck="false"
				autocomplete="off"
				autocorrect="off"
				autocapitalize="off"
			/>
		</div>
	{/if}

	{#if exportError}
		<p class="error-text export-error">{exportError}</p>
	{/if}
	{#if exportResult}
		<div class="export-result-wrap">
			<ExportResultCard result={exportResult} defaultSaveName={exportResult.filename} />
		</div>
	{/if}

	{#if sourcesStore.items.length === 0}
		<p class="empty-state text-muted">No sources yet. Add an RSS feed to start syncing articles.</p>
	{:else if filteredSources.length === 0}
		<p class="empty-state text-muted">No sources match “{search}”.</p>
	{:else}
		{#if activeSources.length > 0}
			<h2 class="group-heading">Active</h2>
			<div class="source-list">
				{#each activeSources as source (source.id)}
					{@render sourceCard(source)}
				{/each}
			</div>
		{/if}
		{#if pausedSources.length > 0}
			<h2 class="group-heading">Paused</h2>
			<div class="source-list">
				{#each pausedSources as source (source.id)}
					{@render sourceCard(source)}
				{/each}
			</div>
		{/if}
	{/if}
</div>

{#snippet sourceCard(source: Source)}
	{@const paused = source.status === 'paused'}
	{@const syncing = syncingIds.has(source.id)}
	<div class="source-card">
		<div class="card-top">
			<div class="avatar">
				{#if source.source_type === 'rss'}
					<Rss size={17} />
				{:else}
					<LinkIcon size={17} />
				{/if}
			</div>
			<div class="heading">
				<div class="name-line">
					<span
						class="status-dot"
						style:background={statusDotColor[source.status]}
						title={statusLabel[source.status]}
					></span>
					<span class="name">{source.name}</span>
				</div>
				<div class="card-meta">
					{typeLabelMap[source.source_type]} · {source.article_count} article{source.article_count === 1
						? ''
						: 's'}
				</div>
			</div>
			<!-- Desktop has enough room to show every card action outright
			     (`.card-desktop-only`) — mobile falls back to the ⋮ overflow menu
			     below it instead. -->
			<div class="card-actions card-desktop-only">
				{#if paused}
					<button class="btn btn-secondary" onclick={() => togglePause(source.id)}>Make active</button>
				{:else}
					<button class="btn btn-secondary" onclick={() => togglePause(source.id)}>Pause</button>
					<button class="btn btn-secondary" disabled={syncing} onclick={() => syncNow(source.id)}>
						<Refresh size={13} spinning={syncing} />
						{syncing ? 'Syncing…' : 'Sync now'}
					</button>
				{/if}
				<button class="btn btn-icon btn-secondary" aria-label={`Remove ${source.name}`} title="Remove" onclick={() => removeSource(source)}>
					<Trash size={14} />
				</button>
			</div>
			<div class="overflow-menu source-menu card-overflow-only" data-id={source.id}>
				<button
					class="btn btn-icon btn-secondary"
					onclick={() => (openMenuId = openMenuId === source.id ? null : source.id)}
					aria-label="Source options"
					aria-expanded={openMenuId === source.id}
				>
					<MoreVertical />
				</button>
				{#if openMenuId === source.id}
					<div class="overflow-popover elev-md" role="menu">
						{#if paused}
							<button class="overflow-item" role="menuitem" onclick={() => togglePause(source.id)}>
								Make active
							</button>
						{:else}
							<button class="overflow-item" role="menuitem" onclick={() => togglePause(source.id)}>
								Pause
							</button>
							<button
								class="overflow-item"
								role="menuitem"
								disabled={syncing}
								onclick={() => syncNow(source.id)}
							>
								{syncing ? 'Syncing…' : 'Sync now'}
							</button>
						{/if}
						<button
							class="overflow-item overflow-item-danger"
							role="menuitem"
							onclick={() => removeSource(source)}
						>
							Remove
						</button>
					</div>
				{/if}
			</div>
		</div>

		{#if source.status === 'error' && source.last_error}
			<p class="error-text card-error">{source.last_error}</p>
		{/if}

		<div class="card-divider"></div>
		<div class="sync-line text-muted">Synced {formatRelativeTime(source.last_synced_at)}</div>
	</div>
{/snippet}

<style>
	.sources-page {
		max-width: 860px;
		padding: 36px 36px 56px;
	}
	.header-row {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 16px;
		flex-wrap: wrap;
		margin-bottom: 18px;
	}
	.header-row h1 {
		font-size: 24px;
		margin: 0;
	}
	.header-actions {
		display: flex;
		align-items: center;
		gap: 8px;
		flex-wrap: wrap;
	}
	.header-actions .btn :global(svg) {
		flex: none;
	}
	/* Mobile keeps the pre-desktop header as it was — icon-only sync button
	   (44px touch target); the label only earns its space on wide screens. */
	@media (max-width: 768px) {
		.sync-all-btn {
			width: 44px;
			padding-inline: 0;
		}
		.sync-all-label {
			display: none;
		}
	}
	/* Wide viewports get every header action as a plain button (see the
	   header markup) — the overflow menu is only for narrow ones. */
	@media (max-width: 768px) {
		.header-desktop-only,
		.card-desktop-only {
			display: none;
		}
	}
	@media (min-width: 769px) {
		.header-overflow-only,
		.card-overflow-only {
			display: none !important;
		}
	}
	.card-actions {
		display: flex;
		align-items: center;
		gap: 8px;
		flex: none;
	}
	.card-actions .btn :global(svg) {
		flex: none;
	}
	.overflow-menu {
		position: relative;
	}
	.overflow-popover {
		position: absolute;
		top: calc(100% + 8px);
		right: 0;
		z-index: 5;
		display: flex;
		flex-direction: column;
		width: 172px;
		padding: var(--space-2);
		border-radius: var(--radius-lg);
		background: var(--color-surface);
	}
	.overflow-item {
		display: flex;
		align-items: center;
		gap: 8px;
		width: 100%;
		text-align: left;
		text-decoration: none;
		background: none;
		border: none;
		border-radius: var(--radius-sm);
		padding: 8px 10px;
		font-family: var(--font-body);
		font-size: 13px;
		color: var(--color-text);
		cursor: pointer;
	}
	.overflow-item:hover:not(:disabled) {
		background: color-mix(in srgb, var(--color-text) 8%, transparent);
	}
	.overflow-item:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}
	.overflow-item-danger {
		color: var(--color-danger);
	}
	.overflow-item-danger:hover:not(:disabled) {
		background: color-mix(in srgb, var(--color-danger) 12%, transparent);
	}
	.search-box {
		display: flex;
		align-items: center;
		gap: 8px;
		background: var(--color-surface);
		border-radius: var(--radius-md);
		padding: 10px 12px;
		margin-bottom: 22px;
		color: var(--color-muted);
	}
	.search-box input {
		border: none;
		background: transparent;
		outline: none;
		font: inherit;
		font-size: 14px;
		width: 100%;
		color: var(--color-text);
	}
	.export-result-wrap {
		margin: -8px 0 18px;
		padding: 12px 14px 4px;
		border-radius: var(--radius-md);
		background: var(--color-surface);
	}
	.export-error {
		margin: -8px 0 18px;
	}
	.group-heading {
		font-family: var(--font-heading);
		font-weight: var(--font-heading-weight);
		font-size: 12px;
		letter-spacing: 0.06em;
		text-transform: uppercase;
		color: var(--color-muted);
		margin: 0 0 10px;
	}
	.source-list {
		display: flex;
		flex-direction: column;
		gap: 14px;
	}
	.source-list + .group-heading {
		margin-top: 26px;
	}
	.source-card {
		padding: 16px 18px 14px;
		border-radius: var(--radius-lg);
		background: var(--color-surface);
		box-shadow: var(--shadow-card);
		transition:
			box-shadow var(--duration-base) var(--ease-snap),
			transform var(--duration-base) var(--ease-snap);
	}
	.source-card:hover {
		box-shadow: var(--shadow-card-hover);
		transform: translateY(-1px);
	}
	.card-top {
		display: flex;
		align-items: flex-start;
		gap: 14px;
	}
	.avatar {
		width: 40px;
		height: 40px;
		flex: none;
		display: flex;
		align-items: center;
		justify-content: center;
		background: var(--color-surface-raised);
		box-shadow: var(--shadow-sm);
		border-radius: 999px;
		color: var(--color-muted);
	}
	.heading {
		flex: 1 1 auto;
		min-width: 0;
		/* Optically centers the (now possibly two-line) heading block against
		   the fixed-height avatar and menu button beside it. */
		padding-top: 3px;
	}
	.name-line {
		display: flex;
		align-items: flex-start;
		gap: 7px;
		min-width: 0;
	}
	.status-dot {
		flex: none;
		width: 7px;
		height: 7px;
		margin-top: 6px;
		border-radius: 50%;
	}
	.name {
		flex: 1 1 auto;
		font-family: var(--font-heading);
		font-weight: var(--font-heading-weight);
		font-size: 15px;
		line-height: 1.3;
		overflow: hidden;
		display: -webkit-box;
		-webkit-line-clamp: 2;
		line-clamp: 2;
		-webkit-box-orient: vertical;
		min-width: 0;
	}
	.card-meta {
		margin-top: 2px;
		font-size: 12px;
		color: var(--color-muted);
	}
	.card-error {
		margin: 8px 0 0;
	}
	.card-divider {
		height: 1px;
		margin: 14px 0 10px;
		background: var(--color-divider);
	}
	.sync-line {
		font-size: 12px;
		margin: 0;
	}
	.error-text {
		margin-top: 3px;
		font-size: 12px;
		color: var(--color-danger);
	}
	.empty-state {
		padding: 40px 0;
	}

	@media (max-width: 768px) {
		.sources-page {
			/* Bottom padding cleared to 104px (not the usual 32px) so the last
			   card isn't hidden behind the floating add-source FAB (see
			   `Shell.svelte`), which overlays every mobile page. */
			padding: calc(20px + env(safe-area-inset-top)) 16px 104px;
		}
	}
</style>
