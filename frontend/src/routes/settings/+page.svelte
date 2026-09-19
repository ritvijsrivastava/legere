<script lang="ts">
	import { onMount } from 'svelte';
	import { settingsStore } from '$lib/stores/settings.svelte';
	import { uiStore } from '$lib/stores/ui.svelte';
	import { importStore } from '$lib/stores/import.svelte';
	import { articleImportStore } from '$lib/stores/articleImport.svelte';
	import ChevronRight from '$lib/icons/ChevronRight.svelte';
	import Download from '$lib/icons/Download.svelte';
	import Upload from '$lib/icons/Upload.svelte';
	import Globe from '$lib/icons/Globe.svelte';
	import FileText from '$lib/icons/FileText.svelte';
	import ExportResultCard from '$lib/components/ExportResultCard.svelte';
	import type { AppTheme, ExportResult, LibraryView, ReaderMeasure } from '$lib/types';
	import { isTauri } from '$lib/platform';
	import { errorMessage } from '$lib/api';
	import * as api from '$lib/api';
	import {
		currentVersion,
		checkForUpdate,
		installUpdate,
		shouldShowLinuxUpdateWarning,
		getReleaseNotes,
		parseChangelog,
		type UpdateCheckResult,
		type UpdateInfo,
		type InstallProgress
	} from '$lib/update';

	const importPct = $derived(
		importStore.total > 0 ? Math.round((importStore.processed / importStore.total) * 100) : 0
	);
	const articleImportPct = $derived(
		articleImportStore.total > 0
			? Math.round((articleImportStore.processed / articleImportStore.total) * 100)
			: 0
	);

	let articlesExportResult = $state<ExportResult | null>(null);
	let exportingArticles = $state(false);
	let articlesExportError = $state('');

	async function exportArticles() {
		exportingArticles = true;
		articlesExportError = '';
		try {
			articlesExportResult = await api.exportArticlesCsv();
		} catch (e) {
			articlesExportError = errorMessage(e);
		} finally {
			exportingArticles = false;
		}
	}

	// The web build has no installer to update, and the underlying Tauri
	// plugin calls throw outside a Tauri context, so gate all of this off.
	const tauri = isTauri();

	let version = $state('');

	let checkState = $state<'idle' | 'checking' | 'error'>('idle');
	let checkError = $state('');
	let updateResult = $state<UpdateCheckResult | null>(null);
	let updateDetailsExpanded = $state(false);
	let updateDetailsState = $state<'idle' | 'loading' | 'loaded' | 'error'>('idle');
	let updateDetailsError = $state('');
	let updateChangelog = $state<string[]>([]);

	let changelogState = $state<'idle' | 'loading' | 'error' | 'none'>('idle');
	let changelogError = $state('');
	let changelog = $state<UpdateInfo | null>(null);

	let installing = $state(false);
	let installPhase = $state<'downloading' | 'installing'>('downloading');
	let installProgress = $state<{ downloaded: number; total: number | null } | null>(null);
	let installError = $state('');
	let installDone = $state(false);

	let showLinuxWarning = $state(false);

	onMount(async () => {
		if (!tauri) return;
		version = await currentVersion();
		shouldShowLinuxUpdateWarning().then((show) => (showLinuxWarning = show));
		handleCheck();
		loadChangelog();
	});

	async function loadChangelog() {
		changelogState = 'loading';
		changelogError = '';
		try {
			const notes = await getReleaseNotes(version);
			changelog = notes;
			changelogState = notes ? 'idle' : 'none';
		} catch (e) {
			changelogError = errorMessage(e);
			changelogState = 'error';
		}
	}

	/** Lazily fetch the real changelog for a pending update the first time
	 *  "Details" is expanded — `updateResult.notes` can't be used here, see
	 *  `getReleaseNotes()`'s doc comment. */
	async function toggleUpdateDetails() {
		updateDetailsExpanded = !updateDetailsExpanded;
		const alreadyFetched = updateDetailsState === 'loading' || updateDetailsState === 'loaded';
		if (!updateDetailsExpanded || alreadyFetched || !updateResult?.available) return;
		updateDetailsState = 'loading';
		try {
			const notes = await getReleaseNotes(updateResult.version);
			updateChangelog = parseChangelog(notes?.notes);
			updateDetailsState = 'loaded';
		} catch (e) {
			updateDetailsError = errorMessage(e);
			updateDetailsState = 'error';
		}
	}

	async function handleCheck() {
		checkState = 'checking';
		checkError = '';
		updateResult = null;
		updateDetailsExpanded = false;
		updateDetailsState = 'idle';
		updateChangelog = [];
		try {
			updateResult = await checkForUpdate();
			checkState = 'idle';
		} catch (e) {
			checkError = errorMessage(e);
			checkState = 'error';
		}
	}

	async function handleInstall() {
		installing = true;
		installPhase = 'downloading';
		installError = '';
		installDone = false;
		installProgress = { downloaded: 0, total: null };
		try {
			await installUpdate((p: InstallProgress) => {
				if (p.event === 'started') {
					installProgress = { downloaded: 0, total: p.data.contentLength };
				} else if (p.event === 'progress') {
					installProgress = {
						downloaded: (installProgress?.downloaded ?? 0) + p.data.chunkLength,
						total: installProgress?.total ?? null
					};
				} else if (p.event === 'installing') {
					installPhase = 'installing';
				} else if (p.event === 'finished') {
					installDone = true;
				}
			});
			// Desktop relaunches automatically inside installUpdate(); Android hands
			// off to the OS package installer. Either way, nothing left to do here.
		} catch (e) {
			installError = errorMessage(e);
		} finally {
			installing = false;
		}
	}

	const installPct = $derived(
		installProgress?.total
			? Math.min(100, Math.round((installProgress.downloaded / installProgress.total) * 100))
			: null
	);

	function setLibraryView(view: LibraryView) {
		settingsStore.update({ default_library_view: view });
	}
	function setAutosync(enabled: boolean) {
		settingsStore.update({ autosync: enabled });
	}
	function setAppTheme(theme: AppTheme) {
		settingsStore.update({ app_theme: theme });
	}
	function setReaderMeasure(measure: ReaderMeasure) {
		settingsStore.update({ reader_measure: measure });
	}
	function setReaderFontSize(delta: number) {
		const next = Math.max(16, Math.min(22, settingsStore.current.reader_font_size + delta));
		settingsStore.update({ reader_font_size: next });
	}
	function setImportConcurrency(value: number) {
		const next = Math.max(5, Math.min(10, Math.round(value)));
		settingsStore.update({ import_concurrency: next });
	}

	let advancedOpen = $state(false);

	const appThemeOptions: { value: AppTheme; label: string }[] = [
		{ value: 'light', label: 'Light' },
		{ value: 'dark', label: 'Dark' }
	];
	const readerMeasureOptions: { value: ReaderMeasure; label: string }[] = [
		{ value: 'narrow', label: 'Narrow' },
		{ value: 'default', label: 'Default' },
		{ value: 'wide', label: 'Wide' }
	];
</script>

<div class="settings-page">
	<h1>Settings</h1>

	<!-- Display & reading -->
	<section class="settings-card">
		<h2 class="group-title">Display &amp; Reading</h2>

		<div class="row row-grouped">
			<span class="row-label">Theme</span>
			<div class="seg seg-compact">
				{#each appThemeOptions as opt (opt.value)}
					<label class="seg-opt">
						<input
							type="radio"
							name="app-theme"
							checked={settingsStore.current.app_theme === opt.value}
							onchange={() => setAppTheme(opt.value)}
						/>
						<span>{opt.label}</span>
					</label>
				{/each}
			</div>
		</div>

		<div class="row row-grouped">
			<span class="row-label">Library layout</span>
			<div class="seg seg-compact">
				<label class="seg-opt">
					<input
						type="radio"
						name="lv2"
						checked={settingsStore.current.default_library_view === 'cards'}
						onchange={() => setLibraryView('cards')}
					/>
					<span>Cards</span>
				</label>
				<label class="seg-opt">
					<input
						type="radio"
						name="lv2"
						checked={settingsStore.current.default_library_view === 'list'}
						onchange={() => setLibraryView('list')}
					/>
					<span>List</span>
				</label>
			</div>
		</div>

		<div class="card-divider"></div>

		<h3 class="row-title">Reader appearance</h3>
		<p class="text-muted section-desc">Override per article from its Aa menu while reading.</p>
		<div class="row">
			<span class="row-label">Text width</span>
			<div class="seg">
				{#each readerMeasureOptions as opt (opt.value)}
					<label class="seg-opt">
						<input
							type="radio"
							name="reader-measure2"
							checked={settingsStore.current.reader_measure === opt.value}
							onchange={() => setReaderMeasure(opt.value)}
						/>
						<span>{opt.label}</span>
					</label>
				{/each}
			</div>
		</div>
		<div class="row">
			<span class="row-label">Font size</span>
			<div class="stepper">
				<button
					class="btn btn-icon btn-secondary"
					onclick={() => setReaderFontSize(-1)}
					disabled={settingsStore.current.reader_font_size <= 16}
					aria-label="Decrease font size"
				>
					–
				</button>
				<span class="stepper-value tabular-nums">{settingsStore.current.reader_font_size}</span>
				<button
					class="btn btn-icon btn-secondary"
					onclick={() => setReaderFontSize(1)}
					disabled={settingsStore.current.reader_font_size >= 22}
					aria-label="Increase font size"
				>
					+
				</button>
			</div>
		</div>
	</section>

	<!-- Content: import, export, sync. Source management lives on its own
	     top-level "Sources" nav destination now (see `Shell.svelte`), so it
	     no longer needs a settings row of its own. -->
	<section class="settings-card zone-start">
		<h2 class="group-title">Sync &amp; Data</h2>

		<div class="row">
			<div class="row-copy">
				<span class="row-label">Auto-fetch articles</span>
				<p class="row-sublabel text-muted">Fetches from all sources while Legere is open.</p>
			</div>
			<div class="seg seg-compact">
				<label class="seg-opt">
					<input
						type="radio"
						name="sync"
						checked={settingsStore.current.autosync}
						onchange={() => setAutosync(true)}
					/>
					<span>On</span>
				</label>
				<label class="seg-opt">
					<input
						type="radio"
						name="sync"
						checked={!settingsStore.current.autosync}
						onchange={() => setAutosync(false)}
					/>
					<span>Off</span>
				</label>
			</div>
		</div>

		{#if !tauri}
			<div class="card-divider"></div>
			<p class="text-muted section-desc">
				Backup, restore, and import aren't available in the web build — install the desktop or
				Android app to back up or import your library.
			</p>
		{:else}
			<div class="card-divider"></div>

			<button class="data-row" onclick={exportArticles} disabled={exportingArticles}>
				<span class="data-row-icon"><Download size={16} /></span>
				<span class="data-row-copy">
					<span class="data-row-title">Export library</span>
					<span class="data-row-sub text-muted">
						{exportingArticles ? 'Exporting…' : 'Save every article, tag, and category to CSV'}
					</span>
				</span>
				<ChevronRight size={14} />
			</button>
			{#if articlesExportError}
				<p class="error-text row-error">{articlesExportError}</p>
			{/if}
			{#if articlesExportResult}
				<div class="row-result">
					<ExportResultCard result={articlesExportResult} defaultSaveName={articlesExportResult.filename} />
				</div>
			{/if}

			<button class="data-row" onclick={() => uiStore.openImportArticlesDialog()}>
				<span class="data-row-icon"><Upload size={16} /></span>
				<span class="data-row-copy">
					<span class="data-row-title">Restore from CSV</span>
					<span class="data-row-sub text-muted">
						{articleImportStore.running ? `Importing… ${articleImportPct}%` : 'Import an existing Legere backup'}
					</span>
				</span>
				<ChevronRight size={14} />
			</button>
			{#if articleImportStore.running}
				<div class="import-progress">
					<div class="import-progress-track">
						<div class="import-progress-fill" style:transform={`scaleX(${articleImportPct / 100})`}></div>
					</div>
					<p class="text-muted import-progress-label">
						{articleImportStore.imported} imported · {articleImportStore.skippedDuplicate} already saved
						· {articleImportStore.failedCount} failed
					</p>
				</div>
			{:else if articleImportStore.finished}
				<div class="import-progress">
					<p class="text-muted import-progress-label">
						Last import{articleImportStore.cancelled ? ' (cancelled)' : ''}: {articleImportStore.imported}
						imported · {articleImportStore.skippedDuplicate} already saved · {articleImportStore.failedCount}
						failed
					</p>
					<div class="import-summary-actions">
						{#if articleImportStore.failedCount > 0}
							<button class="btn btn-ghost import-summary-btn" onclick={() => uiStore.openImportArticlesDialog()}>
								View failed links
							</button>
						{/if}
						<button class="btn btn-ghost import-summary-btn" onclick={() => articleImportStore.dismiss()}>
							Dismiss
						</button>
					</div>
				</div>
			{/if}

			<button class="data-row" onclick={() => uiStore.openImportDialog()}>
				<span class="data-row-icon"><Globe size={16} /></span>
				<span class="data-row-copy">
					<span class="data-row-title">Import from Raindrop.io</span>
					<span class="data-row-sub text-muted">
						{importStore.running ? `Importing… ${importPct}%` : 'Links, folders, and tags'}
					</span>
				</span>
				<ChevronRight size={14} />
			</button>
			{#if importStore.running}
				<div class="import-progress">
					<div class="import-progress-track">
						<div class="import-progress-fill" style:transform={`scaleX(${importPct / 100})`}></div>
					</div>
					<p class="text-muted import-progress-label">
						Importing {importStore.processed} of {importStore.total}
					</p>
					<p class="text-muted import-progress-label">
						{importStore.imported} imported · {importStore.skippedDuplicate} already saved · {importStore.failedCount}
						failed
					</p>
				</div>
			{:else if importStore.finished}
				<div class="import-progress">
					<p class="text-muted import-progress-label">
						Last import{importStore.cancelled ? ' (cancelled)' : ''}: {importStore.imported} imported ·
						{importStore.skippedDuplicate} already saved · {importStore.failedCount} failed
					</p>
					<div class="import-summary-actions">
						{#if importStore.failedCount > 0}
							<button class="btn btn-ghost import-summary-btn" onclick={() => uiStore.openImportDialog()}>
								View failed links
							</button>
						{/if}
						<button class="btn btn-ghost import-summary-btn" onclick={() => importStore.dismiss()}>
							Dismiss
						</button>
					</div>
				</div>
			{/if}

			<div class="card-divider"></div>

			<details class="advanced" bind:open={advancedOpen}>
				<summary class="advanced-summary">
					<span>Advanced data controls</span>
					<span class="advanced-chevron"><ChevronRight size={13} /></span>
				</summary>
				<div class="advanced-body">
					<div class="row">
						<span class="row-label">Concurrent captures ({settingsStore.current.import_concurrency})</span>
					</div>
					<input
						class="slider"
						type="range"
						min="5"
						max="10"
						step="1"
						value={settingsStore.current.import_concurrency}
						oninput={(e) => setImportConcurrency(Number(e.currentTarget.value))}
						aria-label="Concurrent captures"
					/>
					<p class="text-muted section-desc concurrency-desc">
						Higher is faster; lower is gentler on source sites.
					</p>
				</div>
			</details>
		{/if}
	</section>

	<!-- App -->
	<section class="settings-card zone-start">
		<h2 class="group-title">About &amp; Updates</h2>
		{#if !tauri}
			<p class="text-muted section-desc">
				Not available in the web build — install the desktop or Android app to get in-app updates.
			</p>
		{:else}
			<div class="row">
				<span class="row-label">Version</span>
				<span class="version-value text-muted">{version || '—'}</span>
			</div>

			<div class="row">
				<span class="row-label">Check for updates</span>
				<button
					class="btn btn-secondary"
					disabled={checkState === 'checking'}
					onclick={handleCheck}
				>
					{checkState === 'checking' ? 'Checking…' : 'Check now'}
				</button>
			</div>

			{#if checkState === 'error'}
				<p class="error-text">{checkError}</p>
			{:else if updateResult?.available}
				<div class="update-block">
					<p class="update-available">Update available — v{updateResult.version}</p>
					<button class="btn btn-ghost details-btn" onclick={toggleUpdateDetails}>
						{updateDetailsExpanded ? 'Hide details' : 'Details'}
					</button>
					{#if updateDetailsExpanded}
						{#if updateDetailsState === 'loading'}
							<p class="text-muted">Loading…</p>
						{:else if updateDetailsState === 'error'}
							<p class="error-text">{updateDetailsError}</p>
						{:else if updateChangelog.length}
							<ul class="changelog text-muted">
								{#each updateChangelog as change}
									<li>{change}</li>
								{/each}
							</ul>
						{:else if updateResult.notes}
							<p class="changelog-fallback text-muted">{updateResult.notes}</p>
						{:else}
							<p class="text-muted">No changelog available.</p>
						{/if}
					{/if}

					{#if installDone}
						<p class="done-text">Installed — relaunching…</p>
					{:else if installing}
						<div class="progress-wrap">
							<div class="progress-track">
								<div
									class="progress-fill"
									class:indeterminate={installPhase === 'installing'}
									style:transform={`scaleX(${(installPhase === 'installing' ? 100 : (installPct ?? 20)) / 100})`}
								></div>
							</div>
							<p class="text-muted progress-label">
								{installPhase === 'installing'
									? 'Installing… confirm the prompt if one appears'
									: (installPct !== null ? `${installPct}%` : 'Downloading…')}
							</p>
						</div>
					{:else}
						<button class="btn btn-primary install-btn" onclick={handleInstall}>
							Download &amp; install
						</button>
					{/if}

					{#if installError}
						<p class="error-text">{installError}</p>
					{/if}
				</div>
			{:else if updateResult && !updateResult.available}
				<p class="text-muted">You're up to date{version ? ` (v${version})` : ''}.</p>
			{/if}

			{#if showLinuxWarning}
				<p class="text-muted linux-note">
					Legere couldn't detect how it was installed, so auto-update isn't available. Check for
					updates manually, or reinstall from the latest GitHub release.
				</p>
			{/if}

			<div class="card-divider"></div>

			<div class="data-row whats-new">
				<span class="data-row-icon"><FileText size={16} /></span>
				<span class="data-row-copy">
					<span class="data-row-title">What's new</span>
					<span class="data-row-sub text-muted">Notes for {version ? `v${version}` : 'this version'}</span>
				</span>
			</div>
			{#if changelogState === 'loading'}
				<p class="text-muted">Loading…</p>
			{:else if changelogState === 'error'}
				<p class="error-text">{changelogError}</p>
			{:else if changelogState === 'none'}
				<p class="text-muted">No release notes found for v{version}.</p>
			{:else if changelog}
				{#if parseChangelog(changelog.notes).length}
					<ul class="changelog text-muted">
						{#each parseChangelog(changelog.notes) as change}
							<li>{change}</li>
						{/each}
					</ul>
				{:else if changelog.notes}
					<p class="changelog-fallback text-muted">{changelog.notes}</p>
				{:else}
					<p class="text-muted">No changelog available.</p>
				{/if}
			{/if}
		{/if}
	</section>

	<section class="settings-card danger-card">
		<h3 class="danger-heading">Danger zone</h3>
		<p class="text-muted section-desc">
			Sources are kept, but every captured article is gone for good.
		</p>
		<button class="btn btn-danger" onclick={() => uiStore.openDeleteAllArticlesDialog()}>
			Delete all articles
		</button>
	</section>

	<div class="settings-footer">
		<p class="text-muted version">Legere — offline article reader.</p>
	</div>
</div>

<style>
	.settings-page {
		/* Matches the reader's own "default" measure at typical window sizes —
		   reusing an existing comfortable-width token instead of inventing a
		   new one. Widened in steps (below) on genuinely wide windows so the
		   cards don't look stranded as a thin strip in a huge void. Centered
		   either way. */
		max-width: 680px;
		margin-inline: auto;
		padding: 48px 36px 56px;
	}
	.settings-page h1 {
		font-size: 24px;
		margin: 0 0 28px;
	}
	.settings-card {
		margin-top: 18px;
		padding: 18px 22px 20px;
		border-radius: var(--radius-lg);
		background: var(--color-surface);
		box-shadow: var(--shadow-sm);
	}
	.settings-card:first-of-type {
		margin-top: 0;
	}
	.settings-card.zone-start {
		margin-top: 40px;
	}
	.settings-card.danger-card {
		margin-top: 56px;
		box-shadow: 0 0 0 1px color-mix(in srgb, var(--color-danger) 32%, transparent);
	}
	.group-title {
		font-family: var(--font-heading);
		font-weight: var(--font-heading-weight);
		font-size: 13px;
		letter-spacing: 0.06em;
		text-transform: uppercase;
		color: var(--color-muted);
		margin: 0 0 12px;
	}
	.row-title {
		font-size: 14px;
		margin: 0 0 6px;
	}
	.settings-card h3 {
		margin: 0 0 4px;
	}
	.settings-card > .row:first-of-type {
		margin-top: 2px;
	}
	.card-divider {
		height: 1px;
		margin: 10px 0;
		background: var(--color-divider);
	}
	.danger-heading {
		color: var(--color-danger);
	}
	.section-desc {
		font-size: 13px;
		margin: 0 0 10px;
	}
	.concurrency-desc {
		margin: 10px 0 0;
		line-height: 1.5;
	}
	.import-progress {
		margin-top: 10px;
	}
	.import-progress-track {
		height: 6px;
		border-radius: 3px;
		background: var(--color-divider);
		overflow: hidden;
	}
	.import-progress-fill {
		width: 100%;
		height: 100%;
		background: var(--color-accent);
		transform-origin: left;
		transition: transform 0.2s ease;
	}
	.import-progress-label {
		font-size: 12px;
		margin: 6px 0 0;
	}
	.import-summary-actions {
		display: flex;
		flex-wrap: wrap;
		gap: 4px 16px;
		margin-top: 4px;
	}
	.import-summary-btn {
		padding: 4px 0;
		font-size: 12px;
	}
	.row {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 14px;
		padding: 6px 0;
	}
	.row-grouped {
		padding: 7px 0;
	}
	.row-copy {
		display: flex;
		flex-direction: column;
		gap: 2px;
		min-width: 0;
	}
	.row-sublabel {
		font-size: 12px;
		margin: 0;
		line-height: 1.4;
	}
	.row-label {
		font-size: 14px;
	}
	.version-value {
		font-size: 13px;
	}
	.stepper {
		display: flex;
		align-items: center;
		gap: 6px;
	}
	.stepper-value {
		min-width: 22px;
		text-align: center;
		font-size: 12px;
	}
	.seg-compact {
		flex: none;
	}
	/* Icon + title/subtitle list rows — Export/Restore/Raindrop import,
	   What's New. Same idiom, some are buttons (navigate/act on click),
	   the changelog row is a plain static heading reusing the same shape. */
	.data-row {
		display: flex;
		align-items: center;
		gap: 12px;
		width: 100%;
		padding: 11px 10px;
		margin: 2px -10px;
		border: none;
		border-radius: var(--radius-md);
		background: none;
		text-align: left;
		font-family: inherit;
		color: var(--color-text);
		cursor: pointer;
		transition: background var(--duration-base) var(--ease-snap);
	}
	button.data-row:hover:not(:disabled) {
		background: color-mix(in srgb, var(--color-text) 6%, transparent);
	}
	button.data-row:disabled {
		cursor: default;
		opacity: 0.7;
	}
	.data-row.whats-new {
		cursor: default;
	}
	.data-row-icon {
		flex: none;
		width: 34px;
		height: 34px;
		display: flex;
		align-items: center;
		justify-content: center;
		border-radius: 999px;
		background: var(--color-surface-raised);
		box-shadow: var(--shadow-sm);
		color: var(--color-muted);
	}
	.data-row-copy {
		flex: 1 1 auto;
		display: flex;
		flex-direction: column;
		gap: 1px;
		min-width: 0;
	}
	.data-row-title {
		font-family: var(--font-heading);
		font-weight: var(--font-heading-weight);
		font-size: 14px;
	}
	.data-row-sub {
		font-size: 12px;
	}
	.data-row :global(svg:last-child) {
		flex: none;
		color: var(--color-muted);
	}
	.row-error {
		margin: 2px 0 8px;
	}
	.row-result {
		margin: -2px 0 8px;
		padding: 12px 14px 4px;
		border-radius: var(--radius-md);
		background: var(--color-surface-raised);
	}

	.advanced {
		margin-top: 2px;
	}
	.advanced-summary {
		display: flex;
		align-items: center;
		justify-content: space-between;
		list-style: none;
		cursor: pointer;
		font-size: 14px;
		padding: 4px 0;
	}
	.advanced-summary::-webkit-details-marker {
		display: none;
	}
	.advanced-chevron {
		display: inline-flex;
		color: var(--color-muted);
		transition: transform var(--duration-base) var(--ease-snap);
	}
	.advanced[open] .advanced-chevron {
		transform: rotate(90deg);
	}
	.advanced-body {
		padding-top: 10px;
	}
	.slider {
		width: 100%;
		height: 4px;
		margin: 4px 0 0;
		appearance: none;
		background: var(--color-divider);
		border-radius: 2px;
		accent-color: var(--color-accent);
	}
	.slider::-webkit-slider-thumb {
		appearance: none;
		width: 16px;
		height: 16px;
		border-radius: 50%;
		background: var(--color-accent);
		box-shadow: 0 0 0 3px var(--color-surface);
		cursor: pointer;
	}
	.slider::-moz-range-thumb {
		width: 16px;
		height: 16px;
		border: none;
		border-radius: 50%;
		background: var(--color-accent);
		box-shadow: 0 0 0 3px var(--color-surface);
		cursor: pointer;
	}

	.settings-footer {
		margin-top: 32px;
		padding-top: 20px;
		border-top: 1px solid var(--color-divider);
	}
	.version {
		font-size: 12px;
		margin: 0;
	}

	.update-block {
		margin-top: 10px;
		padding-top: 10px;
		border-top: 1px solid var(--color-divider);
	}
	.update-available {
		font-family: var(--font-heading);
		font-weight: var(--font-heading-weight);
		font-size: 14px;
		color: var(--color-accent);
		margin: 0;
	}
	.details-btn {
		margin-top: 6px;
	}
	.install-btn {
		margin-top: 10px;
	}
	.changelog {
		list-style: disc;
		font-size: 12px;
		line-height: 1.6;
		margin: 8px 0 0;
		padding-left: 18px;
	}
	.changelog li {
		margin-bottom: 2px;
	}
	.changelog-fallback {
		font-size: 12px;
		margin: 8px 0 0;
		white-space: pre-wrap;
	}
	.error-text {
		font-size: 12px;
		color: var(--color-danger);
		margin: 6px 0 0;
	}
	.done-text {
		font-size: 12px;
		font-weight: 600;
		color: var(--color-accent);
		margin: 10px 0 0;
	}
	.progress-wrap {
		margin-top: 10px;
	}
	.progress-track {
		height: 5px;
		border-radius: 3px;
		background: var(--color-divider);
		overflow: hidden;
	}
	.progress-fill {
		width: 100%;
		height: 100%;
		background: var(--color-accent);
		transform-origin: left;
		transition: transform 0.2s ease;
	}
	.progress-fill.indeterminate {
		animation: progress-pulse 1.4s ease-in-out infinite;
	}
	@keyframes progress-pulse {
		0%,
		100% {
			opacity: 0.4;
		}
		50% {
			opacity: 1;
		}
	}
	.progress-label {
		font-size: 11px;
		margin: 6px 0 0;
	}
	.linux-note {
		font-size: 12px;
		margin: 10px 0 0;
		padding-top: 10px;
		border-top: 1px solid var(--color-divider);
		line-height: 1.5;
	}

	@media (min-width: 1100px) {
		.settings-page {
			max-width: 800px;
		}
	}
	@media (min-width: 1500px) {
		.settings-page {
			max-width: 920px;
		}
	}

	@media (max-width: 768px) {
		.settings-page {
			/* Bottom padding cleared to 104px (not the usual 32px) so the last
			   card isn't hidden behind the floating add-source FAB (see
			   `Shell.svelte`), which overlays every mobile page. */
			padding: calc(20px + env(safe-area-inset-top)) 16px 104px;
		}
		.settings-card {
			padding: 18px 16px 20px;
		}
		.settings-card.zone-start {
			margin-top: 28px;
		}
		.settings-card.danger-card {
			margin-top: 40px;
		}
	}
</style>
