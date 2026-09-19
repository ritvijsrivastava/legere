<script lang="ts">
	// Desktop already has per-category rename/delete via the settings gear
	// on `/category/[id]` (`CategorySettingsDialog`), reachable because the
	// sidebar links straight to that dedicated page. Mobile's category
	// picker (`MobileCategorySheet`) instead just sets a filter chip and
	// stays on the current view, so that gear button was never reachable
	// from a phone — this page is the fix, mirroring `/tags` (itself linked
	// from `TagBrowser`, shared by the desktop sidebar and the mobile Tags
	// sheet): one management surface, linked from `MobileCategorySheet`
	// for mobile, that both platforms can use.
	import * as api from '$lib/api';
	import Search from '$lib/icons/Search.svelte';
	import Trash from '$lib/icons/Trash.svelte';
	import Plus from '$lib/icons/Plus.svelte';
	import Pencil from '$lib/icons/Pencil.svelte';
	import { libraryStatsStore } from '$lib/stores/libraryStats.svelte';
	import CategoryIconPicker from '$lib/components/CategoryIconPicker.svelte';

	let search = $state('');
	let error = $state<string | null>(null);

	/** The "New category" inline row, toggled by the header button — lives
	 *  above the list rather than a separate dialog since it's just one
	 *  field, same reasoning as the inline rename row below. */
	let creatingOpen = $state(false);
	let newCategoryName = $state('');
	let newCategoryInputEl = $state<HTMLInputElement>();
	let creating = $state(false);

	/** The category currently being renamed, if any — only one row can be
	 *  in edit mode at a time. */
	let renamingId = $state<string | null>(null);
	let renameValue = $state('');
	let renameInputEl = $state<HTMLInputElement>();
	let saving = $state(false);
	let deletingId = $state<string | null>(null);
	let savingIconId = $state<string | null>(null);

	// Uncategorized is a virtual entry (see `libraryStatsStore`) — it can't
	// be renamed or deleted, so it's excluded from this management list
	// entirely rather than shown disabled.
	let categories = $derived(libraryStatsStore.categories.filter((c) => c.id !== '__uncategorized__'));
	let visibleCategories = $derived(
		categories.filter((c) => c.name.toLowerCase().includes(search.trim().toLowerCase()))
	);

	function openCreate() {
		creatingOpen = true;
		newCategoryName = '';
		error = null;
		requestAnimationFrame(() => newCategoryInputEl?.focus());
	}

	function cancelCreate() {
		creatingOpen = false;
		newCategoryName = '';
	}

	async function saveCreate() {
		const name = newCategoryName.trim();
		if (!name || creating) return;
		creating = true;
		error = null;
		try {
			// Emits `category:changed`, which `libraryStatsStore` refreshes
			// from — `categories` above re-derives once that lands, same as
			// every other write on this page.
			await api.createCategory(name);
			// Clears any active filter that would otherwise hide the category
			// just created — there'd be no visible confirmation it worked.
			search = '';
			creatingOpen = false;
			newCategoryName = '';
		} catch (e) {
			error = api.errorMessage(e);
		} finally {
			creating = false;
		}
	}

	function startRename(id: string, name: string) {
		renamingId = id;
		renameValue = name;
		error = null;
		// The input doesn't exist until this render commits.
		requestAnimationFrame(() => renameInputEl?.focus());
	}

	function cancelRename() {
		renamingId = null;
		renameValue = '';
	}

	async function saveRename(id: string, currentName: string) {
		const next = renameValue.trim();
		if (!next || next === currentName || saving) {
			cancelRename();
			return;
		}
		saving = true;
		error = null;
		try {
			// Emits `category:changed`, which `libraryStatsStore` listens for
			// and refreshes from — no local patch needed, `categories` above
			// re-derives once that lands.
			await api.renameCategory(id, next);
			renamingId = null;
			renameValue = '';
		} catch (e) {
			error = api.errorMessage(e);
		} finally {
			saving = false;
		}
	}

	async function changeIcon(id: string, icon: string) {
		savingIconId = id;
		error = null;
		try {
			// Emits `category:changed`, which `libraryStatsStore` refreshes
			// from — same no-local-patch-needed shape as `saveRename` above.
			await api.setCategoryIcon(id, icon);
		} catch (e) {
			error = api.errorMessage(e);
		} finally {
			savingIconId = null;
		}
	}

	async function removeCategory(id: string, name: string, count: number) {
		if (
			!confirm(
				`Delete the category "${name}"? ${count > 0 ? `Its ${count} article${count === 1 ? '' : 's'} will move to Uncategorized.` : 'It has no articles.'}`
			)
		) {
			return;
		}
		deletingId = id;
		error = null;
		try {
			await api.deleteCategory(id);
		} catch (e) {
			error = api.errorMessage(e);
		} finally {
			deletingId = null;
		}
	}
</script>

<div class="categories-page">
	<div class="header-row">
		<div class="title-text">
			<h1>Categories</h1>
			<span class="sub"
				>{categories.length} categor{categories.length === 1 ? 'y' : 'ies'}</span
			>
		</div>
		<div class="header-actions">
			<div class="search-box">
				<Search size={13} />
				<input
					type="text"
					placeholder="Search categories..."
					bind:value={search}
					spellcheck="false"
					autocomplete="off"
					autocorrect="off"
					autocapitalize="off"
				/>
			</div>
			<button class="btn btn-primary" onclick={openCreate}>
				<Plus size={14} />
				New category
			</button>
		</div>
	</div>

	{#if error}<div class="error-text">{error}</div>{/if}

	{#if creatingOpen}
		<div class="category-row create-row">
			<input
				class="input rename-input"
				placeholder="Category name"
				bind:this={newCategoryInputEl}
				bind:value={newCategoryName}
				autocomplete="off"
				disabled={creating}
				onkeydown={(e) => {
					if (e.key === 'Enter') saveCreate();
					if (e.key === 'Escape') cancelCreate();
				}}
			/>
			<span class="row-actions">
				<button class="btn btn-secondary" onclick={cancelCreate} disabled={creating}>
					Cancel
				</button>
				<button
					class="btn btn-primary"
					onclick={saveCreate}
					disabled={creating || !newCategoryName.trim()}
				>
					{creating ? 'Creating…' : 'Create'}
				</button>
			</span>
		</div>
	{/if}

	{#if libraryStatsStore.loaded && categories.length === 0}
		<p class="empty-state text-muted">
			No categories yet. Move an article into a new one from its card or the reader.
		</p>
	{:else if libraryStatsStore.loaded && visibleCategories.length === 0}
		<p class="empty-state text-muted">No categories match "{search}".</p>
	{:else}
		<div class="category-list">
			{#each visibleCategories as category (category.id)}
				<div class="category-row">
					{#if renamingId === category.id}
						<input
							class="input rename-input"
							bind:this={renameInputEl}
							bind:value={renameValue}
							disabled={saving}
							onkeydown={(e) => {
								if (e.key === 'Enter') saveRename(category.id, category.name);
								if (e.key === 'Escape') cancelRename();
							}}
						/>
						<span class="row-actions">
							<button class="btn btn-secondary" onclick={cancelRename} disabled={saving}>
								Cancel
							</button>
							<button
								class="btn btn-primary"
								onclick={() => saveRename(category.id, category.name)}
								disabled={saving || !renameValue.trim() || renameValue.trim() === category.name}
							>
								{saving ? 'Saving…' : 'Save'}
							</button>
						</span>
					{:else}
						<CategoryIconPicker
							icon={category.icon}
							disabled={savingIconId === category.id}
							onselect={(icon) => changeIcon(category.id, icon)}
						/>
						<button class="name" onclick={() => startRename(category.id, category.name)}>
							{category.name}
						</button>
						<span class="count"
							>{category.article_count} article{category.article_count === 1 ? '' : 's'}</span
						>
						<span class="row-actions">
							<button
								class="btn btn-icon btn-secondary"
								aria-label={`Rename category ${category.name}`}
								onclick={() => startRename(category.id, category.name)}
							>
								<Pencil size={15} />
							</button>
							<button
								class="btn btn-icon btn-secondary"
								aria-label={`Delete category ${category.name}`}
								onclick={() => removeCategory(category.id, category.name, category.article_count)}
								disabled={deletingId === category.id}
							>
								<Trash />
							</button>
						</span>
					{/if}
				</div>
			{/each}
		</div>
	{/if}
</div>

<style>
	.categories-page {
		max-width: 860px;
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
	.title-text {
		min-width: 0;
	}
	.header-row h1 {
		font-size: 24px;
		margin: 0;
	}
	/* Search + create sit on the right of the title on desktop; on narrow
	   widths they wrap to their own full-width row (search stretches, the
	   button keeps its size) instead of ordering themselves around the
	   title. */
	.header-actions {
		display: flex;
		align-items: center;
		gap: 14px;
		margin-left: auto;
	}
	.sub {
		font-size: 12px;
		color: var(--color-muted);
	}
	.search-box {
		display: flex;
		align-items: center;
		gap: 8px;
		background: var(--color-surface);
		border-radius: 12px;
		padding: 9px 10px 9px 14px;
		width: 220px;
		color: var(--color-muted);
	}
	.search-box input {
		border: none;
		background: transparent;
		outline: none;
		font: inherit;
		font-size: 13px;
		width: 100%;
		color: var(--color-text);
	}
	.error-text {
		margin-bottom: 16px;
		font-size: 13px;
		color: var(--color-danger);
	}
	.empty-state {
		padding: 40px 0;
	}
	.category-list {
		display: flex;
		flex-direction: column;
		gap: 2px;
	}
	.category-row {
		display: flex;
		align-items: center;
		gap: 14px;
		padding: 10px 8px;
		border-radius: var(--radius-md);
	}
	.category-row:hover {
		background: var(--color-surface);
	}
	.name {
		flex: none;
		background: none;
		border: none;
		cursor: pointer;
		font-family: var(--font-heading);
		font-weight: 600;
		font-size: 14px;
		color: var(--color-text);
		padding: 4px 2px;
		text-align: left;
	}
	.name:hover {
		color: var(--color-accent);
	}
	.count {
		flex: 1;
		font-size: 12px;
		color: var(--color-muted);
	}
	.row-actions {
		display: flex;
		align-items: center;
		gap: 8px;
		flex: none;
	}
	.rename-input {
		flex: 1;
		max-width: 320px;
	}
	.create-row {
		margin-bottom: 10px;
		background: var(--color-surface);
	}

	@media (max-width: 768px) {
		.categories-page {
			/* Bottom padding cleared to 104px (not the usual 32px) so the last
			   row isn't hidden behind the floating add-source FAB (see
			   `Shell.svelte`), which overlays every mobile page. */
			padding: calc(20px + env(safe-area-inset-top)) 16px 104px;
		}
		.header-actions {
			width: 100%;
		}
		.search-box {
			flex: 1;
			width: auto;
		}
	}
</style>
