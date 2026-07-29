<script lang="ts">
	import { uiStore } from '$lib/stores/ui.svelte';
</script>

{#if uiStore.toastMessage}
	<div class="toast elev-md" role="status">
		{uiStore.toastMessage}
	</div>
{/if}

<style>
	.toast {
		position: fixed;
		left: 50%;
		bottom: 24px;
		transform: translateX(-50%);
		z-index: 20;
		max-width: min(90vw, 420px);
		padding: 10px 16px;
		border-radius: var(--radius-md);
		background: var(--color-surface);
		color: var(--color-text);
		font-size: 13px;
		text-align: center;
	}
</style>
