<script lang="ts">
	// Mobile-only bottom sheet giving touch devices the same tag
	// search/browse/select access the desktop sidebar's Tags section has
	// — there's no sidebar on mobile to host it inline. Shares
	// `TagBrowser` with that sidebar section so the search/facet-narrowing
	// logic isn't duplicated.
	import TagBrowser from './TagBrowser.svelte';
	import X from '$lib/icons/X.svelte';

	let { open, onclose }: { open: boolean; onclose: () => void } = $props();
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
			aria-label="Browse tags"
			tabindex="-1"
			onclick={(e) => e.stopPropagation()}
			onkeydown={(e) => e.stopPropagation()}
		>
			<div class="sheet-handle"></div>
			<div class="sheet-header">
				<h2>Tags</h2>
				<button class="btn btn-icon btn-secondary" onclick={onclose} aria-label="Close">
					<X size={15} />
				</button>
			</div>
			<div class="sheet-body">
				<TagBrowser maxVisible={40} onSelect={onclose} />
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
