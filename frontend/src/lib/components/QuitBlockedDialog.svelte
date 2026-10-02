<script lang="ts">
	import { uiStore } from '$lib/stores/ui.svelte';
	import * as api from '$lib/api';

	let quitting = $state(false);

	function label() {
		return uiStore.quitBlockedByImportKind === 'raindrop'
			? 'Raindrop.io import'
			: 'Article import';
	}

	function waitForImport() {
		if (quitting) return;
		uiStore.dismissQuitBlockedByImport();
	}

	async function quitAnyway() {
		if (quitting) return;
		quitting = true;
		await api.forceQuit();
	}
</script>

<svelte:window
	onkeydown={(e) => {
		if (uiStore.quitBlockedByImportKind && e.key === 'Escape') waitForImport();
	}}
/>

{#if uiStore.quitBlockedByImportKind}
	<div class="dialog-backdrop" role="presentation" onclick={waitForImport}>
		<div
			class="dialog"
			role="dialog"
			aria-modal="true"
			aria-labelledby="quit-blocked-title"
			tabindex="-1"
			onclick={(e) => e.stopPropagation()}
			onkeydown={(e) => e.stopPropagation()}
		>
			<div class="dialog-title" id="quit-blocked-title">Import still running</div>
			<div class="dialog-body">
				A {label()} is still in progress. Quitting now stops it partway through — anything
				already saved stays saved, but the rest of the export won't be imported.
			</div>
			<div class="dialog-actions">
				<button class="btn btn-secondary" onclick={waitForImport} disabled={quitting}>
					Wait for it to finish
				</button>
				<button class="btn btn-danger" onclick={quitAnyway} disabled={quitting}>
					{quitting ? 'Quitting\u2026' : 'Quit anyway'}
				</button>
			</div>
		</div>
	</div>
{/if}
