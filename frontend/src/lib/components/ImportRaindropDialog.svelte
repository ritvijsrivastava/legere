<script lang="ts">
	import { uiStore } from '$lib/stores/ui.svelte';
	import { importStore } from '$lib/stores/import.svelte';
	import * as api from '$lib/api';

	let selectedPath = $state<string | null>(null);
	let starting = $state(false);
	let cancelling = $state(false);
	let error = $state<string | null>(null);
	let failuresExpanded = $state(false);
	let exporting = $state(false);

	const selectedFileName = $derived(
		selectedPath ? (selectedPath.split(/[/\\]/).pop() ?? selectedPath) : null
	);
	const progressPct = $derived(
		importStore.total > 0 ? Math.round((importStore.processed / importStore.total) * 100) : 0
	);

	function close() {
		uiStore.closeImportDialog();
		// Only clears the dialog's own local state \u2014 `importStore` itself
		// is left alone so a still-running (or just-finished) import stays
		// visible if the dialog is reopened.
		selectedPath = null;
		error = null;
		failuresExpanded = false;
	}

	async function chooseFile() {
		error = null;
		try {
			const path = await api.pickCsvFile({
				multiple: false,
				filters: [{ name: 'CSV', extensions: ['csv'] }]
			});
			if (typeof path === 'string') selectedPath = path;
		} catch (e) {
			error = api.errorMessage(e);
		}
	}

	async function startImport() {
		if (!selectedPath) return;
		starting = true;
		error = null;
		try {
			await api.importRaindropCsv(selectedPath);
			selectedPath = null;
		} catch (e) {
			error = api.errorMessage(e);
		} finally {
			starting = false;
		}
	}

	// `cancelling` stays true across the whole gap between "user clicked
	// cancel" and "the backend actually stopped" — up to `CONCURRENCY`
	// (5) in-flight captures are allowed to finish before the import
	// really ends (see `raindrop_import::run_import`'s doc comment), which
	// can take several seconds. Resetting `cancelling` as soon as the
	// `cancel_raindrop_import` call itself resolves (it only flips a flag,
	// so that round-trip is near-instant) made the button flip back to
	// "Cancel import" immediately, looking like the click did nothing
	// right up until the import quietly stopped later.
	async function cancelImport() {
		cancelling = true;
		try {
			await api.cancelRaindropImport();
		} catch (e) {
			cancelling = false;
			error = api.errorMessage(e);
		}
	}

	$effect(() => {
		if (!importStore.running) cancelling = false;
	});

	function startOver() {
		importStore.dismiss();
		failuresExpanded = false;
	}

	function failuresToCsv(failures: typeof importStore.failures): string {
		const escape = (value: string) => `"${value.replace(/"/g, '""')}"`;
		const rows = failures.map((f) => [f.title, f.url, f.error].map(escape).join(','));
		return ['Title,URL,Error', ...rows].join('\r\n');
	}

	async function exportFailures() {
		error = null;
		exporting = true;
		try {
			const path = await api.saveCsvFile({
				defaultPath: 'legere-import-failures.csv',
				filters: [{ name: 'CSV', extensions: ['csv'] }]
			});
			if (!path) return;
			await api.writeTextFile(path, failuresToCsv(importStore.failures));
		} catch (e) {
			error = api.errorMessage(e);
		} finally {
			exporting = false;
		}
	}
</script>

<svelte:window
	onkeydown={(e) => {
		if (uiStore.importDialogOpen && e.key === 'Escape') close();
	}}
/>

{#if uiStore.importDialogOpen}
	<div class="dialog-backdrop" role="presentation" onclick={close}>
		<div
			class="dialog"
			role="dialog"
			aria-modal="true"
			aria-labelledby="import-raindrop-title"
			tabindex="-1"
			onclick={(e) => e.stopPropagation()}
			onkeydown={(e) => e.stopPropagation()}
		>
			<div class="dialog-title" id="import-raindrop-title">Import from Raindrop</div>

			{#if importStore.running}
				<div class="dialog-body">
					Importing {importStore.processed} of {importStore.total}
					<div class="progress-track">
						<div class="progress-fill" style:width="{progressPct}%"></div>
					</div>
					<p class="text-muted counts">
						{importStore.imported} imported · {importStore.skippedDuplicate} already saved · {importStore.failedCount}
						failed
					</p>
					{#if cancelling}
						<p class="text-muted counts">Finishing in-flight articles before stopping…</p>
					{/if}
				</div>
				{#if error}
					<div class="dialog-body dialog-body-error">{error}</div>
				{/if}
				<div class="dialog-actions">
					<button class="btn btn-secondary" onclick={close}>Hide</button>
					<button class="btn btn-secondary" disabled={cancelling} onclick={cancelImport}>
						{cancelling ? 'Cancelling\u2026' : 'Cancel import'}
					</button>
				</div>
			{:else if importStore.finished}
				<div class="dialog-body">
					{importStore.cancelled ? 'Import cancelled.' : 'Import finished.'}
					<p class="text-muted counts">
						{importStore.imported} imported · {importStore.skippedDuplicate} already saved · {importStore.failedCount}
						failed
					</p>
					{#if importStore.failures.length > 0}
						<div class="failure-actions">
							<button class="btn btn-ghost details-btn" onclick={() => (failuresExpanded = !failuresExpanded)}>
								{failuresExpanded ? 'Hide failed links' : 'Show failed links'}
							</button>
							<button class="btn btn-ghost details-btn" disabled={exporting} onclick={exportFailures}>
								{exporting ? 'Exporting\u2026' : 'Export failed links as CSV'}
							</button>
						</div>
						{#if failuresExpanded}
							<ul class="failure-list">
								{#each importStore.failures as failure (failure.url)}
									<li>
										<span class="failure-title">{failure.title || failure.url}</span>
										<span class="failure-error text-muted">{failure.error}</span>
									</li>
								{/each}
							</ul>
						{/if}
					{/if}
				</div>
				<div class="dialog-actions">
					<button class="btn btn-secondary" onclick={close}>Close</button>
					<button class="btn btn-primary" onclick={startOver}>Import another file</button>
				</div>
			{:else}
				<div class="field">
					<label for="import-file">Raindrop CSV export</label>
					<button id="import-file" class="btn btn-secondary file-btn" onclick={chooseFile}>
						{selectedFileName ?? 'Choose file\u2026'}
					</button>
				</div>
				<div class="dialog-body">
					Imports each bookmark's link and tags as a new article, captured for offline reading just
					like adding a link manually. Notes, folders, and highlights aren't imported. Links already
					in your library are skipped.
				</div>
				{#if error}
					<div class="dialog-body dialog-body-error">{error}</div>
				{/if}
				<div class="dialog-actions">
					<button class="btn btn-secondary" onclick={close}>Cancel</button>
					<button class="btn btn-primary" onclick={startImport} disabled={!selectedPath || starting}>
						{starting ? 'Starting\u2026' : 'Import'}
					</button>
				</div>
			{/if}
		</div>
	</div>
{/if}

<style>
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
		width: 100%;
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
	.progress-track {
		height: 6px;
		border-radius: 3px;
		background: var(--color-divider);
		overflow: hidden;
		margin-top: 10px;
	}
	.progress-fill {
		height: 100%;
		background: var(--color-accent);
		transition: width 0.2s ease;
	}
	.failure-actions {
		display: flex;
		flex-wrap: wrap;
		gap: 4px 16px;
	}
	.details-btn {
		margin-top: 10px;
		padding-left: 0;
	}
	.failure-list {
		list-style: none;
		margin: 8px 0 0;
		padding: 0;
		max-height: 180px;
		overflow-y: auto;
		border-top: 1px solid var(--color-divider);
	}
	.failure-list li {
		padding: 8px 0;
		border-bottom: 1px solid var(--color-divider);
		font-size: 12px;
		display: flex;
		flex-direction: column;
		gap: 2px;
	}
	.failure-title {
		word-break: break-all;
	}
	.failure-error {
		font-size: 11px;
	}
</style>
