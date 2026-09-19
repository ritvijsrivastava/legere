<script lang="ts">
	import * as api from '$lib/api';
	import Search from '$lib/icons/Search.svelte';
	import Trash from '$lib/icons/Trash.svelte';
	import Pencil from '$lib/icons/Pencil.svelte';
	import type { NamedCount } from '$lib/types';

	let tags = $state<NamedCount[]>([]);
	let loaded = $state(false);
	let search = $state('');
	let error = $state<string | null>(null);

	/** The tag currently being renamed, if any \u2014 only one row can be in
	 *  edit mode at a time. */
	let renamingTag = $state<string | null>(null);
	let renameValue = $state('');
	let renameInputEl = $state<HTMLInputElement>();
	let saving = $state(false);
	let deletingTag = $state<string | null>(null);

	async function load() {
		try {
			tags = await api.listTags();
			error = null;
		} catch (e) {
			error = api.errorMessage(e);
		} finally {
			loaded = true;
		}
	}

	$effect(() => {
		load();
	});

	let visibleTags = $derived(
		tags.filter(([tag]) => tag.includes(search.trim().toLowerCase()))
	);

	function startRename(tag: string) {
		renamingTag = tag;
		renameValue = tag;
		error = null;
		// The input doesn't exist until this render commits.
		requestAnimationFrame(() => renameInputEl?.focus());
	}

	function cancelRename() {
		renamingTag = null;
		renameValue = '';
	}

	async function saveRename(oldTag: string) {
		const next = renameValue.trim().toLowerCase();
		if (!next || next === oldTag || saving) {
			cancelRename();
			return;
		}
		saving = true;
		error = null;
		try {
			await api.renameTag(oldTag, next);
			renamingTag = null;
			renameValue = '';
			await load();
		} catch (e) {
			error = api.errorMessage(e);
		} finally {
			saving = false;
		}
	}

	async function removeTag(tag: string, count: number) {
		if (
			!confirm(
				`Remove the tag "${tag}" from ${count} article${count === 1 ? '' : 's'}? The articles themselves are kept.`
			)
		) {
			return;
		}
		deletingTag = tag;
		error = null;
		try {
			await api.deleteTag(tag);
			await load();
		} catch (e) {
			error = api.errorMessage(e);
		} finally {
			deletingTag = null;
		}
	}
</script>

<div class="tags-page">
	<div class="header-row">
		<div>
			<h1>Tags</h1>
			<span class="sub">{tags.length} tag{tags.length === 1 ? '' : 's'}</span>
		</div>
		<div class="search-box">
			<Search size={13} />
			<input
				type="text"
				placeholder="Search tags..."
				bind:value={search}
				spellcheck="false"
				autocomplete="off"
				autocorrect="off"
				autocapitalize="off"
			/>
		</div>
	</div>

	{#if error}<div class="error-text">{error}</div>{/if}

	{#if loaded && tags.length === 0}
		<p class="empty-state text-muted">
			No tags yet. Tags come from feed categories or the reader's tag editor.
		</p>
	{:else if loaded && visibleTags.length === 0}
		<p class="empty-state text-muted">No tags match "{search}".</p>
	{:else}
		<div class="tag-list">
			{#each visibleTags as [tag, count] (tag)}
				<div class="tag-row">
					{#if renamingTag === tag}
						<input
							class="input rename-input"
							bind:this={renameInputEl}
							bind:value={renameValue}
							disabled={saving}
							onkeydown={(e) => {
								if (e.key === 'Enter') saveRename(tag);
								if (e.key === 'Escape') cancelRename();
							}}
						/>
						<span class="row-actions">
							<button class="btn btn-secondary" onclick={cancelRename} disabled={saving}>
								Cancel
							</button>
							<button
								class="btn btn-primary"
								onclick={() => saveRename(tag)}
								disabled={saving ||
									!renameValue.trim() ||
									renameValue.trim().toLowerCase() === tag}
							>
								{saving ? 'Saving…' : 'Save'}
							</button>
						</span>
					{:else}
						<button class="name" onclick={() => startRename(tag)}>#{tag}</button>
						<span class="count">{count} article{count === 1 ? '' : 's'}</span>
						<span class="row-actions">
							<button
								class="btn btn-icon btn-secondary"
								aria-label={`Rename tag ${tag}`}
								onclick={() => startRename(tag)}
							>
								<Pencil size={15} />
							</button>
							<button
								class="btn btn-icon btn-secondary"
								aria-label={`Delete tag ${tag}`}
								onclick={() => removeTag(tag, count)}
								disabled={deletingTag === tag}
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
	.tags-page {
		max-width: 860px;
		padding: 36px 36px 56px;
	}
	.header-row {
		display: flex;
		align-items: flex-start;
		justify-content: space-between;
		gap: 16px;
		flex-wrap: wrap;
		margin-bottom: 22px;
	}
	.header-row h1 {
		font-size: 24px;
		margin: 0;
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
	.tag-list {
		display: flex;
		flex-direction: column;
		gap: 2px;
	}
	.tag-row {
		display: flex;
		align-items: center;
		gap: 14px;
		padding: 10px 8px;
		border-radius: var(--radius-md);
	}
	.tag-row:hover {
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

	@media (max-width: 768px) {
		.tags-page {
			/* Bottom padding cleared to 104px (not the usual 32px) so the last
			   row isn't hidden behind the floating add-source FAB (see
			   `Shell.svelte`), which overlays every mobile page. */
			padding: calc(20px + env(safe-area-inset-top)) 16px 104px;
		}
		.search-box {
			width: 100%;
		}
	}
</style>
