<script lang="ts">
	import { uiStore } from '$lib/stores/ui.svelte';
	import { importStore } from '$lib/stores/import.svelte';
	import type { Category, FolderResolution, ImportPreview } from '$lib/types';
	import * as api from '$lib/api';

	const UNCATEGORIZED = '__uncategorized__';

	let selectedPath = $state<string | null>(null);
	let preview = $state<ImportPreview | null>(null);
	let categories = $state<Category[]>([]);
	let folderChoices = $state<Record<string, string>>({});
	let conflictChoices = $state<Record<string, 'keep' | 'move' | ''>>({});
	let newCategoryName = $state('');
	let previewing = $state(false);
	let creatingCategory = $state(false);
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
	const choicesComplete = $derived(
		preview !== null && preview.folders.every((folder) => Boolean(folderChoices[folder.folder]))
	);
	const conflictsComplete = $derived(
		preview !== null &&
		preview.folders.every((folder) => !hasConflict(folder.folder) || Boolean(conflictChoices[folder.folder]))
	);
	const canStart = $derived(Boolean(selectedPath && preview && choicesComplete && conflictsComplete));

	function clearSelection() {
		selectedPath = null;
		preview = null;
		categories = [];
		folderChoices = {};
		conflictChoices = {};
		newCategoryName = '';
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
			folderChoices = {};
			conflictChoices = {};
			previewing = true;
			const [nextPreview, nextCategories] = await Promise.all([
				api.previewRaindropCsv(path),
				api.getCategories()
			]);
			preview = nextPreview;
			categories = nextCategories;
			folderChoices = Object.fromEntries(nextPreview.folders.map((folder) => [folder.folder, '']));
		} catch (e) {
			error = api.errorMessage(e);
			preview = null;
		} finally {
			previewing = false;
		}
	}

	function categoryName(categoryId: string): string | null {
		return categories.find((category) => category.id === categoryId)?.name ?? null;
	}

	function hasConflict(folder: string): boolean {
		const folderPreview = preview?.folders.find((item) => item.folder === folder);
		const choice = folderChoices[folder];
		if (!folderPreview || !choice || folderPreview.existing_categories.length === 0) return false;
		if (choice === UNCATEGORIZED) return true;
		const chosenName = categoryName(choice);
		return (
			chosenName === null ||
			folderPreview.existing_categories.some(
				(existing) => existing.toLowerCase() !== chosenName.toLowerCase()
			)
		);
	}

	async function createCategory() {
		const name = newCategoryName.trim();
		if (!name) return;
		creatingCategory = true;
		error = null;
		try {
			const category = await api.createCategory(name);
			categories = [...categories, category].sort((a, b) => a.name.localeCompare(b.name));
			newCategoryName = '';
		} catch (e) {
			error = api.errorMessage(e);
		} finally {
			creatingCategory = false;
		}
	}

	function buildResolutions(): FolderResolution[] {
		if (!preview) return [];
		return preview.folders.map((folder) => {
			const choice = folderChoices[folder.folder];
			return {
				folder: folder.folder,
				category_id: choice === UNCATEGORIZED ? null : choice,
				keep_existing_on_conflict: hasConflict(folder.folder)
					? conflictChoices[folder.folder] === 'keep'
					: false
			};
		});
	}

	async function startImport() {
		if (!selectedPath || !canStart) return;
		starting = true;
		error = null;
		try {
			await api.importRaindropCsv(selectedPath, buildResolutions());
			clearSelection();
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
						<span>Choose a category for each folder before importing.</span>
					</div>

					<div class="folder-list" aria-label="Raindrop folders">
						{#each preview.folders as folder (folder.folder)}
							<section class="folder-row">
								<div class="folder-heading">
									<strong>{folder.name}</strong>
									<span>{folder.row_count} {folder.row_count === 1 ? 'bookmark' : 'bookmarks'}</span>
								</div>
								{#if folder.existing_categories.length > 0}
									<p class="conflict-note">
										Saved duplicates currently use: {folder.existing_categories.join(', ')}.
									</p>
								{/if}
								<label class="folder-select-label" for={`folder-${folder.folder || 'uncategorized'}`}>
									Category
								</label>
								<select
									id={`folder-${folder.folder || 'uncategorized'}`}
									class="input folder-select"
									value={folderChoices[folder.folder] ?? ''}
									onchange={(event) =>
										(folderChoices[folder.folder] = (event.currentTarget as HTMLSelectElement).value)}
								>
									<option value="">Choose a category…</option>
									<option value={UNCATEGORIZED}>Uncategorized</option>
									{#each categories as category (category.id)}
										<option value={category.id}>{category.name}</option>
									{/each}
								</select>

								{#if hasConflict(folder.folder)}
									<div class="conflict-choice">
										<span class="folder-select-label">Duplicate handling</span>
										<label>
											<input
												type="radio"
												name={`conflict-${folder.folder}`}
												value="keep"
												checked={conflictChoices[folder.folder] === 'keep'}
												onchange={() => (conflictChoices[folder.folder] = 'keep')}
											/>
											Keep each saved article’s current category
										</label>
										<label>
											<input
												type="radio"
												name={`conflict-${folder.folder}`}
												value="move"
												checked={conflictChoices[folder.folder] === 'move'}
												onchange={() => (conflictChoices[folder.folder] = 'move')}
											/>
											Move duplicates to the selected category
										</label>
									</div>
								{/if}
							</section>
						{/each}
					</div>

					<div class="new-category">
						<label class="folder-select-label" for="new-import-category">Need another category?</label>
						<div class="new-category-row">
							<input
								id="new-import-category"
								class="input"
								placeholder="Category name"
								bind:value={newCategoryName}
								onkeydown={(event) => event.key === 'Enter' && createCategory()}
							/>
							<button class="btn btn-secondary" onclick={createCategory} disabled={creatingCategory || !newCategoryName.trim()}>
								{creatingCategory ? 'Creating…' : 'Create'}
							</button>
						</div>
					</div>
				{:else}
					<div class="dialog-body">
						Choose a Raindrop CSV to review its folders before anything is captured. Each folder can be assigned to an existing category or left Uncategorized explicitly.
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
		width: min(620px, 94vw);
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
		width: 100%;
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
	.preview-intro span,
	.folder-heading span,
	.conflict-note {
		color: var(--color-muted);
	}
	.folder-list {
		display: flex;
		flex-direction: column;
		gap: 8px;
		max-height: min(390px, 42vh);
		overflow-y: auto;
		padding: 2px;
	}
	.folder-row {
		display: flex;
		flex-direction: column;
		gap: 7px;
		padding: 12px;
		border: 1px solid var(--color-divider);
		border-radius: var(--radius-md);
	}
	.folder-heading {
		display: flex;
		align-items: baseline;
		justify-content: space-between;
		gap: 12px;
		font-size: 14px;
	}
	.folder-heading span,
	.conflict-note,
	.folder-select-label,
	.conflict-choice label {
		font-size: 12px;
	}
	.conflict-note {
		margin: 0;
		line-height: 1.4;
	}
	.folder-select-label {
		color: var(--color-muted);
	}
	.folder-select {
		width: 100%;
	}
	.conflict-choice {
		display: flex;
		flex-direction: column;
		gap: 7px;
		padding-top: 4px;
	}
	.conflict-choice label {
		display: flex;
		align-items: flex-start;
		gap: 7px;
		line-height: 1.35;
	}
	.conflict-choice input {
		accent-color: var(--color-accent);
		margin: 1px 0 0;
	}
	.new-category {
		display: flex;
		flex-direction: column;
		gap: 6px;
	}
	.new-category-row {
		display: flex;
		gap: 8px;
	}
	.new-category-row .input {
		min-width: 0;
		flex: 1;
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
	@media (max-width: 520px) {
		.dialog {
			padding: 20px;
		}
		.folder-heading {
			align-items: flex-start;
			flex-direction: column;
			gap: 2px;
		}
	}
</style>
