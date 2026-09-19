<script lang="ts">
	import { uiStore } from '$lib/stores/ui.svelte';
	import * as api from '$lib/api';

	let selectedPath = $state<string | null>(null);
	let starting = $state(false);
	let error = $state<string | null>(null);
	let result = $state<{ queued: number; skipped_duplicate: number } | null>(null);

	const selectedFileName = $derived(
		selectedPath ? (selectedPath.split(/[/\\]/).pop() ?? selectedPath) : null
	);

	function close() {
		uiStore.closeImportSourcesDialog();
		selectedPath = null;
		error = null;
		result = null;
	}

	async function chooseFile() {
		error = null;
		try {
			const path = await api.pickCsvFile({
				multiple: false,
				filters: [{ name: 'CSV', extensions: ['csv'] }]
			});
			if (typeof path !== 'string') return;
			selectedPath = path;
		} catch (e) {
			error = api.errorMessage(e);
		}
	}

	async function startImport() {
		if (!selectedPath) return;
		starting = true;
		error = null;
		try {
			// Each new feed queues into the same background capture pipeline
			// as "Add a source" — tracked by the activity dock/`capture:*`
			// events, not a dedicated progress view here.
			result = await api.importSourcesCsv(selectedPath);
		} catch (e) {
			error = api.errorMessage(e);
		} finally {
			starting = false;
		}
	}
</script>

<svelte:window
	onkeydown={(e) => {
		if (uiStore.importSourcesDialogOpen && e.key === 'Escape') close();
	}}
/>

{#if uiStore.importSourcesDialogOpen}
	<div class="dialog-backdrop" role="presentation" onclick={close}>
		<div
			class="dialog"
			role="dialog"
			aria-modal="true"
			aria-labelledby="import-sources-title"
			tabindex="-1"
			onclick={(e) => e.stopPropagation()}
			onkeydown={(e) => e.stopPropagation()}
		>
			<div class="dialog-title" id="import-sources-title">Import sources</div>

			{#if result}
				<div class="dialog-body">
					{result.queued} new source{result.queued === 1 ? '' : 's'} queued
					{#if result.skipped_duplicate > 0}
						· {result.skipped_duplicate} already added
					{/if}
					<p class="text-muted counts">
						Each new feed is fetched in the background — track progress from the activity panel.
					</p>
				</div>
				<div class="dialog-actions">
					<button class="btn btn-primary" onclick={close}>Done</button>
				</div>
			{:else}
				<div class="field">
					<label for="import-sources-file">Legere sources CSV</label>
					<button
						id="import-sources-file"
						class="btn btn-secondary file-btn"
						onclick={chooseFile}
					>
						{selectedFileName ?? 'Choose file…'}
					</button>
				</div>
				<div class="dialog-body">
					Choose a CSV exported from Legere (or any file with a <code>feed_url</code> column) — every
					feed not already followed is added and synced in the background.
				</div>
				{#if error}
					<div class="dialog-body dialog-body-error">{error}</div>
				{/if}
				<div class="dialog-actions">
					<button class="btn btn-secondary" onclick={close}>Cancel</button>
					<button class="btn btn-primary" onclick={startImport} disabled={!selectedPath || starting}>
						{starting ? 'Starting…' : 'Import'}
					</button>
				</div>
			{/if}
		</div>
	</div>
{/if}

<style>
	.dialog {
		width: min(480px, 94vw);
	}
	.field {
		margin-bottom: 4px;
	}
	.field label {
		display: block;
		font-size: 12px;
		color: var(--color-muted);
		margin-bottom: 6px;
	}
	.file-btn {
		width: fit-content;
		max-width: 100%;
		justify-content: flex-start;
		text-align: left;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.counts {
		font-size: 12px;
		margin: 10px 0 0;
	}
	@media (max-width: 520px) {
		.dialog {
			padding: 20px;
		}
	}
</style>
