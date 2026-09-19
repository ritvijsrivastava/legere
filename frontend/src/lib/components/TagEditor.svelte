<script lang="ts">
	import * as api from '$lib/api';
	import Plus from '$lib/icons/Plus.svelte';
	import { libraryStatsStore } from '$lib/stores/libraryStats.svelte';

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
	let activeSuggestion = $state(-1);

	// Every tag that exists across the library, minus ones this article
	// already carries — sourced from `libraryStatsStore` (already fetched
	// app-wide, see the sidebar), not a fresh `list_tags` round-trip per
	// keystroke.
	let suggestions = $derived.by(() => {
		const value = draft.trim().toLowerCase();
		const candidates = libraryStatsStore.tags
			.map(([tag]) => tag)
			.filter((tag) => !tags.includes(tag));
		if (!value) return candidates.slice(0, 8);
		return candidates.filter((tag) => tag.includes(value)).slice(0, 8);
	});

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
		activeSuggestion = -1;
		// `inputEl` doesn't exist until the `{#if adding}` block below
		// renders it — wait a tick.
		requestAnimationFrame(() => inputEl?.focus());
	}

	function commitDraft(explicitValue?: string) {
		// Lowercased/trimmed here too so the chip that appears the instant
		// you hit Enter already looks like what the server will echo back
		// (it's the actual source of truth either way) rather than
		// flashing your literal input first.
		const value = (explicitValue ?? draft).trim().toLowerCase();
		draft = '';
		activeSuggestion = -1;
		if (!value || tags.includes(value)) {
			adding = false;
			return;
		}
		save([...tags, value]);
		adding = false;
	}

	function applySuggestion(tag: string) {
		if (saving) return;
		commitDraft(tag);
	}

	function onDraftKeydown(event: KeyboardEvent) {
		if (event.key === 'Enter') {
			event.preventDefault();
			if (activeSuggestion >= 0 && suggestions[activeSuggestion]) {
				commitDraft(suggestions[activeSuggestion]);
			} else {
				commitDraft();
			}
		} else if (event.key === 'Escape') {
			draft = '';
			activeSuggestion = -1;
			adding = false;
		} else if (event.key === 'ArrowDown') {
			if (suggestions.length === 0) return;
			event.preventDefault();
			activeSuggestion = (activeSuggestion + 1) % suggestions.length;
		} else if (event.key === 'ArrowUp') {
			if (suggestions.length === 0) return;
			event.preventDefault();
			activeSuggestion = activeSuggestion <= 0 ? suggestions.length - 1 : activeSuggestion - 1;
		} else {
			activeSuggestion = -1;
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
		<div class="tag-input-wrap">
			<input
				bind:this={inputEl}
				class="tag-input"
				type="text"
				placeholder="tag name"
				bind:value={draft}
				onkeydown={onDraftKeydown}
				onblur={() => commitDraft()}
				disabled={saving}
				role="combobox"
				aria-expanded={suggestions.length > 0}
				aria-autocomplete="list"
				aria-controls="tag-suggestions"
			/>
			{#if suggestions.length > 0}
				<ul class="tag-suggestions" id="tag-suggestions" role="listbox">
					{#each suggestions as suggestion, i (suggestion)}
						<li role="presentation">
							<button
								type="button"
								role="option"
								aria-selected={i === activeSuggestion}
								class="tag-suggestion"
								class:active={i === activeSuggestion}
								onmousedown={(event) => event.preventDefault()}
								onclick={() => applySuggestion(suggestion)}
							>
								#{suggestion}
							</button>
						</li>
					{/each}
				</ul>
			{/if}
		</div>
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
	.tag-input-wrap {
		position: relative;
		display: inline-flex;
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
	.tag-suggestions {
		position: absolute;
		top: calc(100% + 4px);
		left: 0;
		z-index: 20;
		min-width: 160px;
		max-width: 240px;
		max-height: 220px;
		overflow-y: auto;
		margin: 0;
		padding: 4px;
		list-style: none;
		border-radius: 10px;
		border: 1px solid var(--color-divider);
		background: var(--color-surface);
		box-shadow: 0 8px 24px rgba(0, 0, 0, 0.18);
	}
	.tag-suggestion {
		display: block;
		width: 100%;
		text-align: left;
		font-size: 12px;
		padding: 6px 9px;
		border: none;
		border-radius: 6px;
		background: transparent;
		color: var(--color-text);
		cursor: pointer;
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}
	.tag-suggestion:hover,
	.tag-suggestion.active {
		background: color-mix(in srgb, var(--color-accent) 16%, transparent);
	}
</style>
