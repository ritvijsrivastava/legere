<script lang="ts">
	// A small trigger (the category's current icon) that opens a popover
	// grid to pick from — used by both `/categories` (inline, per row) and
	// `CategorySettingsDialog` (desktop's rename/delete dialog, gaining an
	// icon field alongside name). Mirrors `SearchScopeFilter`'s own
	// click-outside/Escape-to-close popover pattern rather than inventing
	// a second one.
	//
	// Two tiers: the curated pack (`categoryIcons.ts`) shows by default —
	// fast, no search needed, covers the common library-topic space. Typing
	// in the search box instead filters the full vendored Lucide set
	// (`lib/lucideIcons.ts`, 1848 icons) by id/label match, each result
	// lazily loading its own icon chunk as it renders (`DynamicIcon`)
	// rather than the picker ever bundling or rendering all 1848 upfront.
	import { CATEGORY_ICONS } from '$lib/categoryIcons';
	import { LUCIDE_ICON_IDS, lucideIconLabel } from '$lib/lucideIcons';
	import DynamicIcon from './DynamicIcon.svelte';
	import CategoryIcon from './CategoryIcon.svelte';

	let {
		icon,
		onselect,
		disabled = false
	}: {
		icon: string;
		onselect: (id: string) => void;
		disabled?: boolean;
	} = $props();

	// Caps how many search results render at once — a filtered list is
	// almost always small (a specific query), but an overly broad query
	// (e.g. a single common letter) shouldn't render/lazy-load hundreds of
	// icons at once.
	const MAX_RESULTS = 90;

	let open = $state(false);
	let query = $state('');
	let rootEl = $state<HTMLElement | null>(null);
	let inputEl = $state<HTMLInputElement | null>(null);

	let results = $derived.by(() => {
		const q = query.trim().toLowerCase();
		if (!q) return null;
		const starts: string[] = [];
		const contains: string[] = [];
		for (const id of LUCIDE_ICON_IDS) {
			if (id === q) {
				starts.unshift(id);
			} else if (id.startsWith(q) || lucideIconLabel(id).toLowerCase().startsWith(q)) {
				starts.push(id);
			} else if (id.includes(q) || lucideIconLabel(id).toLowerCase().includes(q)) {
				contains.push(id);
			}
			if (starts.length + contains.length >= MAX_RESULTS * 4) break;
		}
		return [...starts, ...contains].slice(0, MAX_RESULTS);
	});

	function handleClickOutside(e: MouseEvent) {
		if (open && rootEl && !rootEl.contains(e.target as Node)) {
			close();
		}
	}

	function toggle() {
		open = !open;
		if (open) {
			query = '';
			queueMicrotask(() => inputEl?.focus());
		}
	}

	function close() {
		open = false;
		query = '';
	}

	function choose(id: string) {
		close();
		if (id !== icon) onselect(id);
	}
</script>

<svelte:window
	onclick={handleClickOutside}
	onkeydown={(e) => {
		if (open && e.key === 'Escape') close();
	}}
/>

<div class="icon-picker" bind:this={rootEl}>
	<button
		type="button"
		class="icon-trigger"
		onclick={toggle}
		aria-label="Change category icon"
		aria-expanded={open}
		{disabled}
	>
		<CategoryIcon {icon} size={15} />
	</button>
	{#if open}
		<div class="icon-popover elev-md" role="menu">
			<input
				bind:this={inputEl}
				bind:value={query}
				type="text"
				class="input icon-search"
				placeholder="Search icons…"
				aria-label="Search icons"
			/>
			<div class="icon-grid">
				{#if results}
					{#if results.length === 0}
						<p class="icon-empty">No icons match "{query}"</p>
					{:else}
						{#each results as id (id)}
							<button
								type="button"
								class="icon-option"
								class:active={id === icon}
								role="menuitemradio"
								aria-checked={id === icon}
								aria-label={lucideIconLabel(id)}
								title={lucideIconLabel(id)}
								onclick={() => choose(id)}
							>
								<DynamicIcon {id} size={16} />
							</button>
						{/each}
					{/if}
				{:else}
					{#each CATEGORY_ICONS as entry (entry.id)}
						<button
							type="button"
							class="icon-option"
							class:active={entry.id === icon}
							role="menuitemradio"
							aria-checked={entry.id === icon}
							aria-label={entry.label}
							title={entry.label}
							onclick={() => choose(entry.id)}
						>
							<entry.Icon size={16} />
						</button>
					{/each}
				{/if}
			</div>
		</div>
	{/if}
</div>

<style>
	.icon-picker {
		position: relative;
		display: flex;
		flex: none;
	}
	.icon-trigger {
		display: flex;
		align-items: center;
		justify-content: center;
		width: 30px;
		height: 30px;
		flex: none;
		background: var(--color-surface);
		border: none;
		border-radius: var(--radius-md);
		color: var(--color-text);
		cursor: pointer;
	}
	.icon-trigger:hover {
		color: var(--color-accent);
	}
	.icon-trigger:disabled {
		opacity: 0.5;
		cursor: default;
	}
	.icon-popover {
		position: absolute;
		top: calc(100% + 8px);
		left: 0;
		z-index: 5;
		width: 252px;
		padding: var(--space-2);
		border-radius: var(--radius-lg);
		background: var(--color-surface-raised);
	}
	.icon-search {
		width: 100%;
		margin-bottom: var(--space-2);
		font-size: 0.8125rem;
	}
	.icon-grid {
		display: grid;
		grid-template-columns: repeat(5, 1fr);
		gap: 4px;
		/* Both tiers (curated 44, search results up to MAX_RESULTS) can run
		   taller than most phone screens — cap it and scroll, so the
		   trigger stays anchored and the picker never runs off-screen. */
		max-height: 284px;
		overflow-y: auto;
	}
	.icon-empty {
		grid-column: 1 / -1;
		margin: var(--space-2) 0;
		font-size: 0.8125rem;
		color: var(--color-muted);
		text-align: center;
	}
	.icon-option {
		display: flex;
		align-items: center;
		justify-content: center;
		width: 44px;
		height: 40px;
		background: none;
		border: none;
		border-radius: var(--radius-sm);
		color: var(--color-muted);
		cursor: pointer;
	}
	.icon-option:hover {
		color: var(--color-text);
		background: color-mix(in srgb, var(--color-text) 8%, transparent);
	}
	.icon-option.active {
		color: var(--color-accent);
		background: color-mix(in srgb, var(--color-accent) 14%, transparent);
	}
</style>
