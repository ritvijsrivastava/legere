<script lang="ts">
	import Filter from '$lib/icons/Filter.svelte';
	import Check from '$lib/icons/Check.svelte';
	import type { SearchScope } from '$lib/types';

	let {
		scope = $bindable()
	}: {
		scope: SearchScope;
	} = $props();

	const OPTIONS: { value: SearchScope; label: string }[] = [
		{ value: 'all', label: 'Everything' },
		{ value: 'articles', label: 'Articles only' },
		{ value: 'categories', label: 'Categories only' },
		{ value: 'tags', label: 'Tags only' }
	];

	let open = $state(false);
	let rootEl = $state<HTMLElement | null>(null);

	function handleClickOutside(e: MouseEvent) {
		if (open && rootEl && !rootEl.contains(e.target as Node)) {
			open = false;
		}
	}

	function choose(value: SearchScope) {
		scope = value;
		open = false;
	}
</script>

<svelte:window
	onclick={handleClickOutside}
	onkeydown={(e) => {
		if (open && e.key === 'Escape') open = false;
	}}
/>

<!-- Lives inside `.search-box` (see `ArticleCollection`) — borderless and
     sized to sit flush with the search icon/input rather than the
     standalone `.btn-icon` chrome used elsewhere. -->
<div class="scope-filter" bind:this={rootEl}>
	<button
		class="scope-trigger"
		class:active={scope !== 'all'}
		onclick={() => (open = !open)}
		aria-label="Filter search results"
		aria-expanded={open}
	>
		<Filter size={13} />
	</button>
	{#if open}
		<div class="scope-popover elev-md" role="menu">
			{#each OPTIONS as opt (opt.value)}
				<button
					class="scope-item"
					role="menuitemradio"
					aria-checked={scope === opt.value}
					onclick={() => choose(opt.value)}
				>
					<span class="scope-check">{#if scope === opt.value}<Check size={12} />{/if}</span>
					{opt.label}
				</button>
			{/each}
		</div>
	{/if}
</div>

<style>
	.scope-filter {
		position: relative;
		display: flex;
		flex: none;
	}
	.scope-trigger {
		display: flex;
		align-items: center;
		justify-content: center;
		width: 22px;
		height: 22px;
		flex: none;
		background: none;
		border: none;
		border-radius: var(--radius-sm);
		padding: 0;
		color: var(--color-muted);
		cursor: pointer;
	}
	.scope-trigger:hover {
		color: var(--color-text);
		background: color-mix(in srgb, var(--color-text) 8%, transparent);
	}
	.scope-trigger.active {
		color: var(--color-accent);
		background: color-mix(in srgb, var(--color-accent) 14%, transparent);
	}
	.scope-popover {
		position: absolute;
		top: calc(100% + 8px);
		right: -4px;
		z-index: 5;
		display: flex;
		flex-direction: column;
		width: 180px;
		padding: var(--space-2);
		border-radius: var(--radius-lg);
		background: var(--color-surface);
	}
	.scope-item {
		display: flex;
		align-items: center;
		gap: 8px;
		width: 100%;
		text-align: left;
		background: none;
		border: none;
		border-radius: var(--radius-sm);
		padding: 8px 10px;
		font-family: var(--font-body);
		font-size: 13px;
		color: var(--color-text);
		cursor: pointer;
	}
	.scope-item:hover {
		background: color-mix(in srgb, var(--color-text) 8%, transparent);
	}
	.scope-check {
		display: flex;
		align-items: center;
		justify-content: center;
		width: 12px;
		flex: none;
		color: var(--color-accent);
	}
</style>
