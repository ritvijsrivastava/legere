<script lang="ts">
	import { goto } from '$app/navigation';
	import { uiStore } from '$lib/stores/ui.svelte';
	import { sourcesStore } from '$lib/stores/sources.svelte';
	import type { ArticleSummary, Category } from '$lib/types';
	import * as api from '$lib/api';

	let newSourceValue = $state('');
	let submitting = $state(false);
	let error = $state<string | null>(null);
	let inputEl = $state<HTMLInputElement>();
	let directArticle = $state<ArticleSummary | null>(null);
	let directCategories = $state<Category[]>([]);
	let directCategoryChoice = $state('');

	$effect(() => {
		if (uiStore.addSourceOpen && !directArticle) inputEl?.focus();
	});

	function reset() {
		newSourceValue = '';
		directArticle = null;
		directCategories = [];
		directCategoryChoice = '';
		error = null;
	}

	function close() {
		uiStore.closeAddSource();
		// A direct article that has just been captured remains safely
		// Uncategorized if the user closes this step. There is no hidden
		// "Direct link" category to fall back to.
		reset();
	}

	async function submit() {
		const value = newSourceValue.trim();
		if (!value) return;
		submitting = true;
		error = null;
		try {
			const result = await sourcesStore.addAuto(value);
			if (result.kind === 'rss') {
				close();
				return;
			}

			directArticle = result.value;
			directCategories = await api.getCategories();
		} catch (e) {
			error = api.errorMessage(e);
		} finally {
			submitting = false;
		}
	}

	async function finishDirect(categoryId: string | null) {
		if (!directArticle) return;
		submitting = true;
		error = null;
		try {
			await api.setArticleCategory(directArticle.id, categoryId);
			const articleId = directArticle.id;
			close();
			await goto(`/reader/${articleId}`);
		} catch (e) {
			error = api.errorMessage(e);
		} finally {
			submitting = false;
		}
	}
</script>

<svelte:window
	onkeydown={(e) => {
		if (uiStore.addSourceOpen && e.key === 'Escape') close();
	}}
/>

{#if uiStore.addSourceOpen}
	<div class="dialog-backdrop" role="presentation" onclick={close}>
		<div
			class="dialog"
			role="dialog"
			aria-modal="true"
			aria-labelledby="add-source-title"
			tabindex="-1"
			onclick={(e) => e.stopPropagation()}
			onkeydown={(e) => e.stopPropagation()}
		>
			<div class="dialog-title" id="add-source-title">
				{directArticle ? 'Choose a category' : 'Add a source'}
			</div>

			{#if directArticle}
				<div class="dialog-body direct-article-summary">
					<strong>{directArticle.title}</strong>
					<span>This article was captured. Choose where it belongs, or leave it Uncategorized.</span>
				</div>
				<div class="field">
					<label for="direct-category">Category</label>
					<select id="direct-category" class="input" bind:value={directCategoryChoice}>
						<option value="">Choose a category…</option>
						<option value="__uncategorized__">Uncategorized</option>
						{#each directCategories as category (category.id)}
							<option value={category.id}>{category.name}</option>
						{/each}
					</select>
				</div>
			{:else}
				<div class="field">
					<label for="new-source-value">Feed or article URL</label>
					<input
						id="new-source-value"
						class="input"
						type="text"
						placeholder="https://example.com/feed-or-article"
						autocomplete="off"
						spellcheck="false"
						bind:this={inputEl}
						bind:value={newSourceValue}
						onkeydown={(e) => {
							if (e.key === 'Enter' && !submitting && newSourceValue.trim()) submit();
						}}
					/>
				</div>
				<div class="dialog-body">
					Legere figures out which kind of link this is. A feed is checked periodically for new
					entries; anything else is captured once as a standalone article.
				</div>
			{/if}

			{#if error}
				<div class="dialog-body dialog-body-error">{error}</div>
			{/if}
			<div class="dialog-actions">
				{#if directArticle}
					<button class="btn btn-secondary" onclick={() => finishDirect(null)} disabled={submitting}>
						{submitting ? 'Saving…' : 'Leave Uncategorized'}
					</button>
					<button
						class="btn btn-primary"
						onclick={() => finishDirect(directCategoryChoice === '__uncategorized__' ? null : directCategoryChoice)}
						disabled={submitting || !directCategoryChoice}
					>
						Save category
					</button>
				{:else}
					<button class="btn btn-secondary" onclick={close}>Cancel</button>
					<button class="btn btn-primary" onclick={submit} disabled={submitting || !newSourceValue.trim()}>
						{submitting ? 'Adding…' : 'Add source'}
					</button>
				{/if}
			</div>
		</div>
	</div>
{/if}

<style>
	.direct-article-summary {
		display: flex;
		flex-direction: column;
		gap: 5px;
	}
	.direct-article-summary strong {
		line-height: 1.35;
	}
	.direct-article-summary span {
		color: var(--color-muted);
		font-size: 13px;
		line-height: 1.45;
	}
</style>
