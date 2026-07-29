<script lang="ts">
	import { sourcesStore } from '$lib/stores/sources.svelte';
	import { uiStore } from '$lib/stores/ui.svelte';
	import Plus from '$lib/icons/Plus.svelte';
	import Rss from '$lib/icons/Rss.svelte';
	import LinkIcon from '$lib/icons/LinkIcon.svelte';
	import Trash from '$lib/icons/Trash.svelte';
	import Refresh from '$lib/icons/Refresh.svelte';
	import { formatRelativeTime } from '$lib/format';
	import type { Source } from '$lib/types';

	$effect(() => {
		sourcesStore.refresh();
	});

	let syncingIds = $state(new Set<string>());

	const typeLabelMap: Record<Source['source_type'], string> = {
		rss: 'RSS feed',
		direct: 'Direct link'
	};

	const statusStyle: Record<Source['status'], { bg: string; color: string; label: string }> = {
		active: { bg: 'var(--color-accent-800)', color: 'var(--color-accent-200)', label: 'Active' },
		paused: { bg: 'var(--color-neutral-800)', color: 'var(--color-neutral-300)', label: 'Paused' },
		error: { bg: 'var(--color-accent-800)', color: 'var(--color-accent-100)', label: 'Error' }
	};

	async function syncNow(id: string) {
		syncingIds = new Set(syncingIds).add(id);
		try {
			await sourcesStore.syncOne(id);
		} finally {
			const next = new Set(syncingIds);
			next.delete(id);
			syncingIds = next;
		}
	}

	async function removeSource(source: Source) {
		if (!confirm(`Remove "${source.name}"? Articles already saved from it are kept.`)) return;
		await sourcesStore.remove(source.id);
	}
</script>

<div class="sources-page">
	<div class="header-row">
		<h1>Sources</h1>
		<button class="btn btn-primary" onclick={() => uiStore.openAddSource()}>
			<Plus size={14} />
			Add source
		</button>
	</div>

	{#if sourcesStore.items.length === 0}
		<p class="empty-state text-muted">No sources yet. Add an RSS feed to start syncing articles.</p>
	{:else}
		<div class="source-list">
			{#each sourcesStore.items as source (source.id)}
				{@const style = statusStyle[source.status]}
				{@const syncing = syncingIds.has(source.id)}
				<div class="source-row">
					<div class="type-icon">
						{#if source.source_type === 'rss'}
							<Rss size={16} />
						{:else}
							<LinkIcon size={16} />
						{/if}
					</div>
					<div class="info">
						<div class="name">{source.name}</div>
						<div class="card-meta">
							{typeLabelMap[source.source_type]} · {source.article_count} articles · synced {formatRelativeTime(
								source.last_synced_at
							)}
						</div>
						{#if source.status === 'error' && source.last_error}
							<div class="error-text">{source.last_error}</div>
						{/if}
					</div>
					<span class="tag" style:background={style.bg} style:color={style.color}>{style.label}</span>
					<button
						class="btn btn-icon btn-secondary"
						aria-label={source.status === 'error' ? 'Retry sync' : 'Sync now'}
						onclick={() => syncNow(source.id)}
						disabled={syncing || source.status === 'paused'}
					>
						<Refresh spinning={syncing} />
					</button>
					<button class="btn btn-secondary" onclick={() => sourcesStore.togglePause(source.id)}>
						{source.status === 'paused' ? 'Resume' : 'Pause'}
					</button>
					<button
						class="btn btn-icon btn-secondary"
						aria-label="Remove"
						onclick={() => removeSource(source)}
					>
						<Trash />
					</button>
				</div>
			{/each}
		</div>
	{/if}
</div>

<style>
	.sources-page {
		max-width: 860px;
		margin: 0 auto;
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
		font-size: 30px;
		font-weight: 500;
		margin: 0;
	}
	.source-list {
		display: flex;
		flex-direction: column;
		gap: 2px;
	}
	.source-row {
		display: flex;
		align-items: center;
		gap: 14px;
		padding: 12px 8px;
		border-radius: var(--radius-md);
	}
	.type-icon {
		width: 34px;
		height: 34px;
		flex: none;
		display: flex;
		align-items: center;
		justify-content: center;
		background: var(--color-surface);
		border-radius: var(--radius-md);
		color: var(--color-neutral-400);
	}
	.info {
		flex: 1;
		min-width: 0;
	}
	.name {
		font-family: var(--font-heading);
		font-weight: 500;
		font-size: 14px;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.error-text {
		margin-top: 3px;
		font-size: 11px;
		color: var(--color-accent-300);
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.empty-state {
		padding: 40px 0;
	}
</style>
