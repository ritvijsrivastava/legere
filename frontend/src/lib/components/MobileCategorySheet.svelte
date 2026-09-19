<script lang="ts">
	// Mobile-only bottom sheet browsing every category in one scrollable
	// list instead of an ever-growing row of inline chips — the same
	// clutter problem `MobileTagSheet` solves for tags, mirrored here for
	// categories. `libraryStatsStore.categories` already carries the
	// virtual "Uncategorized" entry first (see that store), so this just
	// renders it like any other row.
	import Search from '$lib/icons/Search.svelte';
	import X from '$lib/icons/X.svelte';
	import { libraryFiltersStore } from '$lib/stores/libraryFilters.svelte';
	import { libraryStatsStore } from '$lib/stores/libraryStats.svelte';
	import CategoryIcon from './CategoryIcon.svelte';

	let { open, onclose }: { open: boolean; onclose: () => void } = $props();

	let query = $state('');
	let categories = $derived(libraryStatsStore.categories);
	let visibleCategories = $derived(
		categories.filter((c) => c.name.toLowerCase().includes(query.trim().toLowerCase()))
	);
	// Excludes the virtual Uncategorized entry — it can't be renamed or
	// deleted, so it doesn't count toward "how many categories can I manage".
	let manageableCount = $derived(categories.filter((c) => c.id !== '__uncategorized__').length);

	function select(id: string | null) {
		libraryFiltersStore.categoryId = id;
		onclose();
	}
</script>

<svelte:window
	onkeydown={(e) => {
		if (open && e.key === 'Escape') onclose();
	}}
/>

{#if open}
	<div class="sheet-backdrop" onclick={onclose} role="presentation">
		<div
			class="sheet"
			role="dialog"
			aria-modal="true"
			aria-label="Browse categories"
			tabindex="-1"
			onclick={(e) => e.stopPropagation()}
			onkeydown={(e) => e.stopPropagation()}
		>
			<div class="sheet-handle"></div>
			<div class="sheet-header">
				<h2>Categories</h2>
				<button class="btn btn-icon btn-secondary" onclick={onclose} aria-label="Close">
					<X size={15} />
				</button>
			</div>
			<div class="sheet-body">
				{#if categories.length > 8}
					<div class="cat-search">
						<Search size={13} />
						<input
							type="text"
							placeholder="Search categories..."
							bind:value={query}
							spellcheck="false"
							autocomplete="off"
							autocorrect="off"
							autocapitalize="off"
						/>
					</div>
				{/if}
				<div class="cat-list">
					<button
						class="cat-row"
						class:active={!libraryFiltersStore.categoryId}
						onclick={() => select(null)}
					>
						<span class="row-label">All</span>
					</button>
					{#each visibleCategories as category (category.id)}
						<button
							class="cat-row"
							class:active={libraryFiltersStore.categoryId === category.id}
							onclick={() => select(category.id)}
						>
							<CategoryIcon icon={category.icon} size={13} />
							<span class="row-label">{category.name}</span>
							<span class="nav-count">{category.article_count}</span>
						</button>
					{/each}
					{#if visibleCategories.length === 0}
						<div class="cat-empty">No categories match</div>
					{/if}
				</div>
				<a href="/categories" class="manage-categories-link" onclick={onclose}>
					<span>Manage categories</span>
					<span class="nav-count">{manageableCount}</span>
				</a>
			</div>
		</div>
	</div>
{/if}

<style>
	.sheet-backdrop {
		position: fixed;
		inset: 0;
		z-index: 30;
		display: flex;
		align-items: flex-end;
		justify-content: center;
		background: color-mix(in srgb, var(--color-text) 45%, transparent);
		backdrop-filter: blur(2px);
		animation: sheet-fade var(--duration-base) var(--ease-snap);
	}
	.sheet {
		width: 100%;
		max-height: min(72vh, 560px);
		display: flex;
		flex-direction: column;
		background: var(--color-surface-raised);
		border-radius: 20px 20px 0 0;
		box-shadow: var(--shadow-lg);
		padding: 10px 20px calc(20px + env(safe-area-inset-bottom));
		animation: sheet-up var(--duration-base) var(--ease-snap);
	}
	.sheet-handle {
		width: 36px;
		height: 4px;
		border-radius: 999px;
		background: var(--color-divider-strong);
		margin: 0 auto 14px;
		flex: none;
	}
	.sheet-header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		margin-bottom: 14px;
		flex: none;
	}
	.sheet-header h2 {
		font-size: 19px;
		margin: 0;
	}
	.sheet-body {
		overflow-y: auto;
		min-height: 0;
	}
	.cat-search {
		display: flex;
		align-items: center;
		gap: 7px;
		background: var(--color-surface);
		border-radius: 10px;
		padding: 10px 12px;
		margin: 0 0 10px;
		color: var(--color-muted);
	}
	.cat-search input {
		border: none;
		background: transparent;
		outline: none;
		font: inherit;
		font-size: 14px;
		width: 100%;
		color: var(--color-text);
	}
	.cat-list {
		display: flex;
		flex-direction: column;
		gap: 1px;
	}
	.cat-row {
		display: flex;
		align-items: center;
		gap: 9px;
		width: 100%;
		background: transparent;
		border: none;
		border-radius: 10px;
		padding: 11px 10px;
		cursor: pointer;
		font-family: var(--font-body);
		font-size: 14px;
		color: var(--color-text);
		text-align: left;
	}
	.cat-row.active {
		background: color-mix(in srgb, var(--color-accent) 14%, var(--color-surface));
		color: var(--color-accent);
		font-weight: 600;
	}
	.cat-row .row-label {
		flex: 1;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.nav-count {
		font-size: 11.5px;
		font-family: var(--font-body);
		color: var(--color-muted);
	}
	.cat-empty {
		padding: 10px;
		font-size: 12.5px;
		color: var(--color-muted);
	}
	.manage-categories-link {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 8px;
		margin: 10px 2px 0;
		padding: 11px 10px;
		border-radius: 10px;
		font-size: 13.5px;
		font-weight: 500;
		color: var(--color-text);
		text-decoration: none;
		flex: none;
	}
	.manage-categories-link:hover,
	.manage-categories-link:active {
		background: var(--color-surface);
		color: var(--color-accent);
	}
	@keyframes sheet-fade {
		from {
			opacity: 0;
		}
	}
	@keyframes sheet-up {
		from {
			transform: translateY(16px);
			opacity: 0;
		}
	}
</style>
