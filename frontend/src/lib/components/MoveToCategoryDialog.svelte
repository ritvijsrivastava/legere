<script lang="ts">
	import { uiStore } from '$lib/stores/ui.svelte';
	import * as api from '$lib/api';
	import type { Category } from '$lib/types';

	let categories = $state<Category[]>([]);
	let search = $state('');
	let creating = $state(false);
	let moving = $state<string | null>(null);
	let error = $state<string | null>(null);
	let inputEl = $state<HTMLInputElement>();

	let article = $derived(uiStore.moveCategoryArticle);
	let open = $derived(article !== null);

	// Uncategorized is a virtual row, not a real `Category` — folded into
	// one filterable/scrollable list instead of a separate always-shown
	// pill so the list stays one shape no matter how many real categories
	// exist (see below: this is the whole point of this component).
	let allRows = $derived([{ id: null, name: 'Uncategorized' }, ...categories]);

	// One filter box instead of a search input plus a pill wall: typing
	// narrows a scrollable list (bounded height, so 5 categories or 500
	// look the same), and an exact case-insensitive match is never
	// ambiguous with "create new" below it.
	let filtered = $derived(
		search.trim()
			? allRows.filter((c) => c.name.toLowerCase().includes(search.trim().toLowerCase()))
			: allRows
	);
	let exactMatch = $derived(
		filtered.some((c) => c.name.toLowerCase() === search.trim().toLowerCase())
	);
	let showCreateRow = $derived(search.trim().length > 0 && !exactMatch);

	$effect(() => {
		if (!article) return;
		search = '';
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
		const name = search.trim();
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
				<label for="move-search">Category</label>
				<input
					id="move-search"
					class="input"
					placeholder="Search or create a category"
					autocomplete="off"
					bind:this={inputEl}
					bind:value={search}
					onkeydown={(e) => {
						if (e.key !== 'Enter' || creating || moving) return;
						if (showCreateRow) createAndMove();
						else if (filtered.length > 0) moveTo(filtered[0].id);
					}}
				/>
			</div>

			<ul class="category-list">
				{#if showCreateRow}
					<li>
						<button class="category-row create-row" disabled={creating} onclick={createAndMove}>
							<span>Create “{search.trim()}”</span>
							<span class="category-row-action">{creating ? 'Creating…' : 'Create & move'}</span>
						</button>
					</li>
				{/if}
				{#each filtered as category (category.id ?? '__uncategorized__')}
					<li>
						<button
							class="category-row"
							disabled={moving !== null}
							onclick={() => moveTo(category.id)}
						>
							<span>{category.name}</span>
							{#if moving === (category.id ?? '__uncategorized__')}
								<span class="category-row-action">Moving…</span>
							{/if}
						</button>
					</li>
				{/each}
				{#if filtered.length === 0 && !showCreateRow}
					<li class="category-empty">No categories yet.</li>
				{/if}
			</ul>

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
	.category-list {
		list-style: none;
		margin: 0;
		padding: 0;
		max-height: 240px;
		overflow-y: auto;
		border: 1px solid var(--color-divider);
		border-radius: var(--radius-md);
	}
	.category-row {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 8px;
		width: 100%;
		padding: 9px 12px;
		border: none;
		border-bottom: 1px solid var(--color-divider);
		background: none;
		color: var(--color-text);
		font-size: 13.5px;
		font-family: var(--font-body);
		text-align: left;
		cursor: pointer;
		overflow: hidden;
	}
	.category-row span:first-child {
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.category-list li:last-child .category-row {
		border-bottom: none;
	}
	.category-row:hover:not(:disabled) {
		background: color-mix(in srgb, var(--color-text) 6%, transparent);
	}
	.category-row:disabled {
		cursor: not-allowed;
		opacity: 0.6;
	}
	.category-row-action {
		flex: none;
		font-size: 12px;
		color: var(--color-muted);
	}
	.create-row span:first-child {
		font-weight: 600;
	}
	.category-empty {
		padding: 12px;
		font-size: 13px;
		color: var(--color-muted);
	}
</style>
