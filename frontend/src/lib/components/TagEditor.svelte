<script lang="ts">
	import * as api from '$lib/api';
	import Plus from '$lib/icons/Plus.svelte';

	// Controlled like `ReaderControls`: this component owns only the
	// add/remove UI and the round-trip to `set_article_tags`, the parent
	// (the reader) owns `article.tags` itself and is told about the result
	// via `onChange` — `set_article_tags` is the actual source of truth
	// (it normalizes lowercase/trimmed/deduped regardless of what's sent),
	// so `onChange` is always called with *its* response, not the locally
	// computed candidate list.
	let {
		articleId,
		tags,
		onChange
	}: {
		articleId: string;
		tags: string[];
		onChange: (tags: string[]) => void;
	} = $props();

	let adding = $state(false);
	let draft = $state('');
	let saving = $state(false);
	let inputEl = $state<HTMLInputElement | null>(null);

	async function save(next: string[]) {
		saving = true;
		try {
			onChange(await api.setArticleTags(articleId, next));
		} finally {
			saving = false;
		}
	}

	function removeTag(tag: string) {
		if (saving) return;
		save(tags.filter((t) => t !== tag));
	}

	function startAdding() {
		adding = true;
		draft = '';
		// `inputEl` doesn't exist until the `{#if adding}` block below
		// renders it — wait a tick.
		requestAnimationFrame(() => inputEl?.focus());
	}

	function commitDraft() {
		// Lowercased/trimmed here too so the chip that appears the instant
		// you hit Enter already looks like what the server will echo back
		// (it's the actual source of truth either way) rather than
		// flashing your literal input first.
		const value = draft.trim().toLowerCase();
		draft = '';
		if (!value || tags.includes(value)) {
			adding = false;
			return;
		}
		save([...tags, value]);
		adding = false;
	}

	function onDraftKeydown(event: KeyboardEvent) {
		if (event.key === 'Enter') {
			event.preventDefault();
			commitDraft();
		} else if (event.key === 'Escape') {
			draft = '';
			adding = false;
		}
	}
</script>

<div class="tag-editor">
	{#each tags as tag (tag)}
		<span class="tag-chip">
			#{tag}
			<button
				type="button"
				class="tag-remove"
				disabled={saving}
				onclick={() => removeTag(tag)}
				aria-label={`Remove tag ${tag}`}
			>
				×
			</button>
		</span>
	{/each}

	{#if adding}
		<input
			bind:this={inputEl}
			class="tag-input"
			type="text"
			placeholder="tag name"
			bind:value={draft}
			onkeydown={onDraftKeydown}
			onblur={commitDraft}
			disabled={saving}
		/>
	{:else}
		<button type="button" class="tag-add" onclick={startAdding} disabled={saving}>
			<Plus size={12} />
			Add tag
		</button>
	{/if}
</div>

<style>
	.tag-editor {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: 6px;
	}
	.tag-chip {
		display: inline-flex;
		align-items: center;
		gap: 5px;
		font-size: 11px;
		padding: 5px 8px 5px 11px;
		border-radius: 999px;
		background: color-mix(in srgb, var(--reader-fg, var(--color-text)) 8%, transparent);
		color: var(--reader-muted, var(--color-muted));
	}
	.tag-remove {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		width: 14px;
		height: 14px;
		line-height: 1;
		font-size: 13px;
		padding: 0;
		border: none;
		background: transparent;
		color: inherit;
		cursor: pointer;
		border-radius: 999px;
		opacity: 0.7;
	}
	.tag-remove:hover:not(:disabled) {
		opacity: 1;
		background: color-mix(in srgb, var(--reader-fg, var(--color-text)) 14%, transparent);
	}
	.tag-remove:disabled {
		cursor: not-allowed;
	}
	.tag-add {
		display: inline-flex;
		align-items: center;
		gap: 4px;
		font-size: 11px;
		padding: 5px 10px;
		border-radius: 999px;
		border: 1px dashed var(--reader-divider, var(--color-divider));
		background: transparent;
		color: var(--reader-muted, var(--color-muted));
		cursor: pointer;
	}
	.tag-add:hover:not(:disabled) {
		color: var(--reader-fg, var(--color-text));
		border-color: var(--reader-fg, var(--color-text));
	}
	.tag-input {
		width: 120px;
		font-size: 11px;
		padding: 5px 11px;
		border-radius: 999px;
		border: 1px solid var(--color-accent);
		background: var(--color-surface);
		color: var(--color-text);
	}
	.tag-input:focus-visible {
		outline: none;
	}
</style>
