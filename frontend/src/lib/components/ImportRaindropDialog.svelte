<script lang="ts">
	import { uiStore } from '$lib/stores/ui.svelte';
	import { importStore } from '$lib/stores/import.svelte';
	import type { ImportPreview } from '$lib/types';
	import * as api from '$lib/api';

	let selectedPath = $state<string | null>(null);
	let preview = $state<ImportPreview | null>(null);
	let previewing = $state(false);
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
	const canStart = $derived(Boolean(selectedPath && preview));

	function clearSelection() {
		selectedPath = null;
		preview = null;
		previewing = false;
	}

	function close() {
		uiStore.closeImportDialog();
		// Only clears the dialog's local selection — `importStore` itself is
		// left alone so a still-running (or just-finished) import stays
		// visible if the dialog is reopened.
		clearSelection();
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
			if (typeof path !== 'string') return;

			selectedPath = path;
			preview = null;
			previewing = true;
			preview = await api.previewRaindropCsv(path);
		} catch (e) {
			error = api.errorMessage(e);
			preview = null;
		} finally {
			previewing = false;
		}
	}

	async function startImport() {
		if (!selectedPath || !canStart) return;
		starting = true;
		error = null;
		try {
			await api.importRaindropCsv(selectedPath);
			// The import now runs in the background (tracked by `importStore`,
			// driven by `import:*` events) — close immediately rather than
			// switching this dialog to a progress view. Settings shows progress
			// inline and its button reopens this same dialog to monitor/cancel.
			close();
		} catch (e) {
			error = api.errorMessage(e);
		} finally {
			starting = false;
		}
	}

	// `cancelling` stays true across the whole gap between "user clicked
	// cancel" and the backend actually stopped — up to CONCURRENCY (5)
	// in-flight captures are allowed to finish before the import really ends.
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
		clearSelection();
		failuresExpanded = false;
		error = null;
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
						<div class="progress-fill" style:transform={`scaleX(${progressPct / 100})`}></div>
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
						{cancelling ? 'Cancelling…' : 'Cancel import'}
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
								{exporting ? 'Exporting…' : 'Export failed links as CSV'}
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
					<button id="import-file" class="btn btn-secondary file-btn" onclick={chooseFile} disabled={previewing}>
						{previewing ? 'Reading export…' : selectedFileName ?? 'Choose file…'}
					</button>
				</div>

				{#if previewing}
					<div class="preview-status" role="status">Reading the export and checking saved articles…</div>
			{:else if preview}
					<div class="preview-intro">
						<strong>{preview.total} bookmarks found</strong>
						<span>
							Each Raindrop folder becomes its own category automatically. Links already saved are
							skipped and left as-is. New tags on those links are still merged in.
						</span>
					</div>

					<ul class="folder-list" aria-label="Raindrop folders">
						{#each preview.folders as folder (folder.folder)}
							<li class="folder-row">
								<strong>{folder.name}</strong>
								<span class="text-muted">
									{folder.row_count - folder.duplicate_count} new
									· {folder.duplicate_count} already saved
								</span>
							</li>
						{/each}
					</ul>
				{:else}
					<div class="dialog-body">
						Choose a Raindrop CSV to review what it contains before anything is captured. Each
						folder in the export becomes its own category; links already saved are skipped.
					</div>
				{/if}

				{#if error}
					<div class="dialog-body dialog-body-error">{error}</div>
				{/if}
				<div class="dialog-actions">
					<button class="btn btn-secondary" onclick={close}>Cancel</button>
					<button class="btn btn-primary" onclick={startImport} disabled={!canStart || starting}>
						{starting ? 'Starting…' : 'Import'}
					</button>
				</div>
			{/if}
		</div>
	</div>
{/if}

<style>
	.dialog {
		width: min(560px, 94vw);
		max-height: min(760px, calc(100vh - 32px));
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
	.preview-status,
	.preview-intro {
		display: flex;
		flex-direction: column;
		gap: 4px;
		font-size: 13px;
	}
	.preview-status {
		padding: 12px;
		border: 1px solid var(--color-divider);
		border-radius: var(--radius-md);
		color: var(--color-muted);
	}
	.preview-intro strong {
		font-size: 15px;
	}
	.preview-intro span {
		color: var(--color-muted);
	}
	.folder-list {
		list-style: none;
		margin: 0;
		padding: 0 2px;
		max-height: min(390px, 42vh);
		overflow-y: auto;
		border-top: 1px solid var(--color-divider);
	}
	.folder-row {
		display: flex;
		align-items: baseline;
		justify-content: space-between;
		gap: 12px;
		padding: 10px 2px;
		border-bottom: 1px solid var(--color-divider);
		font-size: 14px;
	}
	.folder-row:last-child {
		border-bottom: none;
	}
	.folder-row span {
		font-size: 12px;
		flex: none;
		color: var(--color-muted);
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
		width: 100%;
		height: 100%;
		background: var(--color-accent);
		transform-origin: left;
		transition: transform 0.2s ease;
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
	@media (max-width: 520px) {
		.dialog {
			padding: 20px;
		}
		.folder-row {
			align-items: flex-start;
			flex-direction: column;
			gap: 2px;
			padding: 10px 0;
		}
	}
</style>
