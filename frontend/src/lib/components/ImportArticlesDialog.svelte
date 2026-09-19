<script lang="ts">
	import { uiStore } from '$lib/stores/ui.svelte';
	import { articleImportStore } from '$lib/stores/articleImport.svelte';
	import type { ArticleImportPreview } from '$lib/types';
	import * as api from '$lib/api';

	let selectedPath = $state<string | null>(null);
	let preview = $state<ArticleImportPreview | null>(null);
	let previewing = $state(false);
	let starting = $state(false);
	let cancelling = $state(false);
	let error = $state<string | null>(null);
	let failuresExpanded = $state(false);

	const selectedFileName = $derived(
		selectedPath ? (selectedPath.split(/[/\\]/).pop() ?? selectedPath) : null
	);
	const progressPct = $derived(
		articleImportStore.total > 0
			? Math.round((articleImportStore.processed / articleImportStore.total) * 100)
			: 0
	);
	const canStart = $derived(Boolean(selectedPath && preview));

	function clearSelection() {
		selectedPath = null;
		preview = null;
		previewing = false;
	}

	function close() {
		uiStore.closeImportArticlesDialog();
		// Only clears the dialog's local selection — the store itself is left
		// alone so a still-running (or just-finished) import stays visible if
		// the dialog is reopened.
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
			preview = await api.previewArticlesCsv(path);
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
			await api.importArticlesCsv(selectedPath);
			close();
		} catch (e) {
			error = api.errorMessage(e);
		} finally {
			starting = false;
		}
	}

	async function cancelImport() {
		cancelling = true;
		try {
			await api.cancelArticlesImport();
		} catch (e) {
			cancelling = false;
			error = api.errorMessage(e);
		}
	}

	$effect(() => {
		if (!articleImportStore.running) cancelling = false;
	});

	function startOver() {
		articleImportStore.dismiss();
		clearSelection();
		failuresExpanded = false;
		error = null;
	}
</script>

<svelte:window
	onkeydown={(e) => {
		if (uiStore.importArticlesDialogOpen && e.key === 'Escape') close();
	}}
/>

{#if uiStore.importArticlesDialogOpen}
	<div class="dialog-backdrop" role="presentation" onclick={close}>
		<div
			class="dialog"
			role="dialog"
			aria-modal="true"
			aria-labelledby="import-articles-title"
			tabindex="-1"
			onclick={(e) => e.stopPropagation()}
			onkeydown={(e) => e.stopPropagation()}
		>
			<div class="dialog-title" id="import-articles-title">Import articles</div>

			{#if articleImportStore.running}
				<div class="dialog-body">
					Importing {articleImportStore.processed} of {articleImportStore.total}
					<div class="progress-track">
						<div class="progress-fill" style:transform={`scaleX(${progressPct / 100})`}></div>
					</div>
					<p class="text-muted counts">
						{articleImportStore.imported} imported · {articleImportStore.skippedDuplicate} already
						saved · {articleImportStore.failedCount} failed
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
			{:else if articleImportStore.finished}
				<div class="dialog-body">
					{articleImportStore.cancelled ? 'Import cancelled.' : 'Import finished.'}
					<p class="text-muted counts">
						{articleImportStore.imported} imported · {articleImportStore.skippedDuplicate} already
						saved · {articleImportStore.failedCount} failed
					</p>
					{#if articleImportStore.failures.length > 0}
						<div class="failure-actions">
							<button class="btn btn-ghost details-btn" onclick={() => (failuresExpanded = !failuresExpanded)}>
								{failuresExpanded ? 'Hide failed links' : 'Show failed links'}
							</button>
						</div>
						{#if failuresExpanded}
							<ul class="failure-list">
								{#each articleImportStore.failures as failure (failure.url)}
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
					<label for="import-articles-file">Legere articles CSV</label>
					<button
						id="import-articles-file"
						class="btn btn-secondary file-btn"
						onclick={chooseFile}
						disabled={previewing}
					>
						{previewing ? 'Reading file…' : (selectedFileName ?? 'Choose file…')}
					</button>
				</div>

				{#if previewing}
					<div class="preview-status" role="status">Reading the file and checking saved articles…</div>
				{:else if preview}
					<div class="preview-intro">
						<strong>{preview.total} links found</strong>
						<span>
							Each category becomes its own folder automatically. Links already saved are
							skipped and left as-is. New tags on those links are still merged in.
						</span>
					</div>

					<ul class="folder-list" aria-label="Categories">
						{#each preview.categories as category (category.category)}
							<li class="folder-row">
								<strong>{category.name}</strong>
								<span class="text-muted">
									{category.row_count - category.duplicate_count} new
									· {category.duplicate_count} already saved
								</span>
							</li>
						{/each}
					</ul>
				{:else}
					<div class="dialog-body">
						Choose a CSV exported from Legere (or any file with the same
						<code>title,url,category,tags,created,favourite</code> columns) to review what it
						contains before anything is captured.
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
