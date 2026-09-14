<script lang="ts">
	import { uiStore } from '$lib/stores/ui.svelte';
	import * as api from '$lib/api';
	import type { Category } from '$lib/types';

	let categories = $state<Category[]>([]);
	let newName = $state('');
	let creating = $state(false);
	let moving = $state<string | null>(null);
	let error = $state<string | null>(null);
	let inputEl = $state<HTMLInputElement>();

	let article = $derived(uiStore.moveCategoryArticle);
	let open = $derived(article !== null);

	$effect(() => {
		if (!article) return;
		newName = '';
		error = null;
		api
			.getCategories()
			.then((items) => (categories = items.sort((a, b) => a.name.localeCompare(b.name))))
			.catch((e) => (error = api.errorMessage(e)));
		inputEl?.focus();
	});

	function close() {
		if (creating || moving) return;
		uiStore.closeMoveCategory();
	}

	async function moveTo(categoryId: string | null) {
		if (!article) return;
		moving = categoryId ?? '__uncategorized__';
		error = null;
		try {
			await api.setArticleCategory(article.id, categoryId);
			uiStore.notifyArticleMoved();
			uiStore.closeMoveCategory();
		} catch (e) {
			error = api.errorMessage(e);
		} finally {
			moving = null;
		}
	}

	/** A name that collides case-insensitively with an existing category
	 *  (rejected by the backend's unique index) just moves the article into
	 *  that existing category instead of surfacing an error \u2014 that's
	 *  obviously what "create" a duplicate-named category means here, and
	 *  it saves a trip through the list below. */
	async function createAndMove() {
		const name = newName.trim();
		if (!article || !name) return;
		creating = true;
		error = null;
		try {
			const created = await api.createCategory(name);
			categories = [...categories, created].sort((a, b) => a.name.localeCompare(b.name));
			await moveTo(created.id);
		} catch (e) {
			const existing = categories.find((c) => c.name.toLowerCase() === name.toLowerCase());
			if (existing) {
				await moveTo(existing.id);
			} else {
				error = api.errorMessage(e);
			}
		} finally {
			creating = false;
		}
	}
</script>

<svelte:window
	onkeydown={(e) => {
		if (open && e.key === 'Escape') close();
	}}
/>

{#if article}
	<div class="dialog-backdrop" role="presentation" onclick={close}>
		<div
			class="dialog"
			role="dialog"
			aria-modal="true"
			aria-labelledby="move-category-title"
			tabindex="-1"
			onclick={(e) => e.stopPropagation()}
			onkeydown={(e) => e.stopPropagation()}
		>
			<div class="dialog-title" id="move-category-title">Move to category</div>
			<div class="dialog-body move-article-title">{article.title}</div>

			<div class="field">
				<label for="move-new-category">New category</label>
				<div class="new-category-row">
					<input
						id="move-new-category"
						class="input"
						list="move-category-suggestions"
						placeholder="Name a new or existing category"
						autocomplete="off"
						bind:this={inputEl}
						bind:value={newName}
						onkeydown={(e) => {
							if (e.key === 'Enter' && !creating && newName.trim()) createAndMove();
						}}
					/>
					<datalist id="move-category-suggestions">
						{#each categories as category (category.id)}
							<option value={category.name}></option>
						{/each}
					</datalist>
					<button
						class="btn btn-secondary"
						onclick={createAndMove}
						disabled={creating || !newName.trim()}
					>
						{creating ? 'Moving…' : 'Create & move'}
					</button>
				</div>
			</div>

			<div class="field">
				<span class="suggestions-label">Or move into an existing category</span>
				<div class="category-suggestions">
					<button
						class="category-option"
						disabled={moving !== null}
						onclick={() => moveTo(null)}
					>
						{moving === '__uncategorized__' ? 'Moving…' : 'Uncategorized'}
					</button>
					{#each categories as category (category.id)}
						<button
							class="category-option"
							disabled={moving !== null}
							onclick={() => moveTo(category.id)}
						>
							{moving === category.id ? 'Moving…' : category.name}
						</button>
					{/each}
				</div>
			</div>

			{#if error}<div class="dialog-body dialog-body-error">{error}</div>{/if}
			<div class="dialog-actions">
				<button class="btn btn-secondary" onclick={close} disabled={creating || moving !== null}>
					Cancel
				</button>
			</div>
		</div>
	</div>
{/if}

<style>
	.move-article-title {
		font-weight: 600;
		opacity: 1;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.new-category-row {
		display: flex;
		gap: 8px;
	}
	.new-category-row .input {
		min-width: 0;
		flex: 1;
	}
	.suggestions-label {
		font-size: 13px;
		color: var(--color-muted);
		display: block;
		margin-bottom: 6px;
	}
	.category-suggestions {
		display: flex;
		flex-wrap: wrap;
		gap: 6px;
	}
	.category-option {
		font-size: 12.5px;
		padding: 7px 13px;
		border-radius: 999px;
		background: var(--color-surface);
		color: var(--color-text);
		border: none;
		cursor: pointer;
		font-family: var(--font-body);
	}
	.category-option:hover:not(:disabled) {
		background: color-mix(in srgb, var(--color-text) 10%, var(--color-surface));
	}
	.category-option:disabled {
		opacity: 0.6;
		cursor: not-allowed;
	}
</style>
