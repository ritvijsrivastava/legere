<script lang="ts">
	import { goto } from '$app/navigation';
	import { articlesStore } from '$lib/stores/articles.svelte';
	import { settingsStore } from '$lib/stores/settings.svelte';
	import { sourcesStore } from '$lib/stores/sources.svelte';
	import { uiStore } from '$lib/stores/ui.svelte';
	import SegmentedControl from '$lib/components/SegmentedControl.svelte';
	import ArticleCard from '$lib/components/ArticleCard.svelte';
	import ArticleListRow from '$lib/components/ArticleListRow.svelte';
	import Grid from '$lib/icons/Grid.svelte';
	import ListIcon from '$lib/icons/ListIcon.svelte';
	import Refresh from '$lib/icons/Refresh.svelte';
	import * as api from '$lib/api';
	import type { LibraryView } from '$lib/types';

	let libraryView = $state<LibraryView>('cards');
	let initializedFromSettings = false;

	$effect(() => {
		if (settingsStore.loaded && !initializedFromSettings) {
			libraryView = settingsStore.current.default_library_view;
			initializedFromSettings = true;
		}
	});

	function setView(view: string) {
		libraryView = view as LibraryView;
		if (initializedFromSettings) {
			settingsStore.update({ default_library_view: libraryView });
		}
	}

	function openArticle(id: string) {
		goto(`/reader/${id}`);
	}

	async function deleteArticle(article: { id: string; title: string }) {
		if (!confirm(`Delete "${article.title}"? This can't be undone.`)) return;
		await api.deleteArticle(article.id);
		articlesStore.items = articlesStore.items.filter((a) => a.id !== article.id);
	}
</script>

<div class="library-page">
	<div class="header-row">
		<div>
			<h1>Library</h1>
			<span class="unread-sub">{articlesStore.unreadCount} unread</span>
		</div>
		<div class="header-controls">
			<button
				class="btn btn-icon btn-secondary"
				onclick={() => sourcesStore.syncAll()}
				disabled={uiStore.syncing}
				aria-label="Refresh"
			>
				<Refresh spinning={uiStore.syncing} />
			</button>
			<div class="seg view-toggle">
				<label class="seg-opt">
					<input
						type="radio"
						name="libview"
						checked={libraryView === 'cards'}
						onchange={() => setView('cards')}
					/>
					<Grid />
				</label>
				<label class="seg-opt">
					<input
						type="radio"
						name="libview"
						checked={libraryView === 'list'}
						onchange={() => setView('list')}
					/>
					<ListIcon />
				</label>
			</div>
		</div>
	</div>

	{#if articlesStore.items.length === 0}
		<p class="empty-state text-muted">
			No articles yet. Add a source or paste a direct link to get started.
		</p>
	{:else if libraryView === 'cards'}
		<div class="cards-grid">
			{#each articlesStore.items as article (article.id)}
				<ArticleCard
					{article}
					onclick={() => openArticle(article.id)}
					ondelete={() => deleteArticle(article)}
				/>
			{/each}
		</div>
	{:else}
		<div class="list-rows">
			{#each articlesStore.items as article (article.id)}
				<ArticleListRow
					{article}
					onclick={() => openArticle(article.id)}
					ondelete={() => deleteArticle(article)}
				/>
			{/each}
		</div>
	{/if}
</div>

<style>
	.library-page {
		max-width: 1040px;
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
	.unread-sub {
		font-size: 12px;
		color: var(--color-neutral-500);
	}
	.header-controls {
		display: flex;
		align-items: center;
		gap: 8px;
	}
	.view-toggle :global(svg) {
		display: block;
	}
	.cards-grid {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(260px, 1fr));
		gap: 16px;
	}
	.list-rows {
		display: flex;
		flex-direction: column;
		gap: 2px;
	}
	.empty-state {
		padding: 40px 0;
	}
</style>
