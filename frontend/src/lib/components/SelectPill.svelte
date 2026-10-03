<script lang="ts" generics="T extends string">
	import ChevronDown from '$lib/icons/ChevronDown.svelte';

	let {
		value,
		options,
		ariaLabel,
		onChange
	}: {
		value: T;
		options: { value: T; label: string }[];
		ariaLabel: string;
		onChange: (value: T) => void;
	} = $props();

	let open = $state(false);
	let rootEl = $state<HTMLElement | null>(null);
	let selectedLabel = $derived(options.find((o) => o.value === value)?.label ?? '');

	function choose(next: T) {
		open = false;
		if (next !== value) onChange(next);
	}

	function handleClickOutside(e: MouseEvent) {
		if (open && rootEl && !rootEl.contains(e.target as Node)) {
			open = false;
		}
	}
</script>

<svelte:window
	onclick={handleClickOutside}
	onkeydown={(e) => {
		if (open && e.key === 'Escape') open = false;
	}}
/>

<!-- A pill-shaped custom listbox, not a native `<select>`: a native
     element's open-panel chrome (hover/selected-row color, box, arrow
     position) is OS-drawn and only partly stylable from CSS, which
     left it looking like a foreign control dropped into this dark
     theme. Same trigger+panel+click-outside shape as the "Aa" popover
     this lives inside, just one level smaller. -->
<div class="select-pill" bind:this={rootEl}>
	<button
		type="button"
		class="select-pill-trigger"
		aria-haspopup="listbox"
		aria-expanded={open}
		aria-label={ariaLabel}
		onclick={() => (open = !open)}
	>
		<span>{selectedLabel}</span>
		<ChevronDown size={12} />
	</button>
	{#if open}
		<ul class="select-pill-menu elev-md" role="listbox" aria-label={ariaLabel}>
			{#each options as opt (opt.value)}
				<li role="presentation">
					<button
						type="button"
						role="option"
						aria-selected={opt.value === value}
						class="select-pill-opt"
						class:selected={opt.value === value}
						onclick={() => choose(opt.value)}
					>
						{opt.label}
					</button>
				</li>
			{/each}
		</ul>
	{/if}
</div>

<style>
	.select-pill {
		position: relative;
		display: inline-flex;
	}
	.select-pill-trigger {
		display: flex;
		align-items: center;
		gap: 6px;
		padding: 5px 11px 5px 13px;
		font: inherit;
		font-size: 12.5px;
		font-weight: 500;
		color: var(--color-text);
		background: transparent;
		border: 1px solid var(--color-divider);
		border-radius: 999px;
		cursor: pointer;
		transition: background var(--duration-base) var(--ease-snap);
	}
	.select-pill-trigger:hover {
		background: color-mix(in srgb, var(--color-text) 7%, transparent);
	}
	.select-pill-trigger:focus-visible {
		outline: none;
		box-shadow: inset 0 0 0 2px var(--color-accent);
	}
	.select-pill-trigger :global(svg) {
		color: color-mix(in srgb, var(--color-text) 55%, transparent);
		flex-shrink: 0;
	}
	.select-pill-menu {
		position: absolute;
		top: calc(100% + 6px);
		right: 0;
		z-index: 1;
		display: flex;
		flex-direction: column;
		gap: 1px;
		min-width: 100%;
		width: max-content;
		margin: 0;
		padding: 4px;
		list-style: none;
		background: var(--color-surface-raised);
		border-radius: var(--radius-md);
	}
	.select-pill-opt {
		display: block;
		width: 100%;
		padding: 6px 10px;
		font: inherit;
		font-size: 13px;
		text-align: left;
		color: var(--color-text);
		background: transparent;
		border: none;
		border-radius: calc(var(--radius-md) - 3px);
		cursor: pointer;
		transition: background var(--duration-base) var(--ease-snap), color var(--duration-base) var(--ease-snap);
	}
	.select-pill-opt:hover {
		background: color-mix(in srgb, var(--color-text) 8%, transparent);
	}
	.select-pill-opt.selected {
		background: var(--color-accent);
		color: var(--color-accent-fg);
		font-weight: 600;
	}
	.select-pill-opt:focus-visible {
		outline: none;
		box-shadow: inset 0 0 0 2px var(--color-accent);
	}
</style>
