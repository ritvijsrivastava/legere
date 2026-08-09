<script lang="ts">
	import MoreVertical from '$lib/icons/MoreVertical.svelte';
	import ExternalLink from '$lib/icons/ExternalLink.svelte';

	let {
		recapturing,
		link,
		onRecapture,
		onDelete
	}: {
		recapturing: boolean;
		link: string;
		onRecapture: () => void;
		onDelete: () => void;
	} = $props();

	let open = $state(false);
	let rootEl = $state<HTMLElement | null>(null);

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

<div class="overflow-menu" bind:this={rootEl}>
	<button
		class="btn btn-icon btn-secondary"
		onclick={() => (open = !open)}
		aria-label="More actions"
		aria-expanded={open}
	>
		<MoreVertical />
	</button>
	{#if open}
		<div class="overflow-popover elev-md" role="menu">
			<a
				class="overflow-item"
				role="menuitem"
				href={link}
				target="_blank"
				rel="noopener"
				onclick={() => (open = false)}
			>
				<ExternalLink />
				View original
			</a>
			<button
				class="overflow-item"
				role="menuitem"
				disabled={recapturing}
				onclick={() => {
					open = false;
					onRecapture();
				}}
			>
				{recapturing ? 'Re-capturing…' : 'Re-capture'}
			</button>
			<button
				class="overflow-item danger"
				role="menuitem"
				onclick={() => {
					open = false;
					onDelete();
				}}
			>
				Delete article
			</button>
		</div>
	{/if}
</div>

<style>
	.overflow-menu {
		position: relative;
	}
	.overflow-popover {
		position: absolute;
		top: calc(100% + 8px);
		right: 0;
		z-index: 5;
		display: flex;
		flex-direction: column;
		width: 180px;
		padding: var(--space-2);
		border-radius: var(--radius-lg);
		background: var(--color-surface);
	}
	.overflow-item {
		display: flex;
		align-items: center;
		gap: 8px;
		width: 100%;
		text-align: left;
		text-decoration: none;
		background: none;
		border: none;
		border-radius: var(--radius-sm);
		padding: 8px 10px;
		font-family: var(--font-body);
		font-size: 13px;
		color: var(--color-text);
		cursor: pointer;
	}
	.overflow-item:hover:not(:disabled) {
		background: color-mix(in srgb, var(--color-text) 8%, transparent);
	}
	.overflow-item:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}
	.overflow-item.danger {
		color: var(--color-danger);
	}
</style>
