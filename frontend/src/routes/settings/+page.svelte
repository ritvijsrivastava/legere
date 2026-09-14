<script lang="ts">
	import { onMount } from 'svelte';
	import { settingsStore } from '$lib/stores/settings.svelte';
	import { sourcesStore } from '$lib/stores/sources.svelte';
	import { uiStore } from '$lib/stores/ui.svelte';
	import { importStore } from '$lib/stores/import.svelte';
	import ChevronRight from '$lib/icons/ChevronRight.svelte';
	import type { AppTheme, LibraryView, ReaderMeasure } from '$lib/types';
	import { isTauri } from '$lib/platform';
	import { errorMessage } from '$lib/api';
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

	$effect(() => {
		sourcesStore.refresh();
	});

	const importPct = $derived(
		importStore.total > 0 ? Math.round((importStore.processed / importStore.total) * 100) : 0
	);

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
	function setImportConcurrency(delta: number) {
		const next = Math.max(5, Math.min(10, settingsStore.current.import_concurrency + delta));
		settingsStore.update({ import_concurrency: next });
	}

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

	<section>
		<h4>Appearance</h4>
		<p class="text-muted section-desc">
			Applies everywhere, immediately. Override it for one article from its Aa menu while
			reading — that doesn't change this.
		</p>
		<div class="seg">
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
	</section>

	<section>
		<h4>Reading defaults</h4>
		<p class="text-muted section-desc">
			Default text settings for articles you open. Override them for one article from its Aa
			menu while reading — that doesn't change this.
		</p>
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
				<span class="stepper-value">{settingsStore.current.reader_font_size}</span>
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

	<section>
		<h4>Library</h4>
		<p class="text-muted section-desc">Default view for the library.</p>
		<div class="seg">
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
	</section>

	<section>
		<h4>Sources</h4>
		<a href="/sources" class="sources-link">
			<span>Manage sources</span>
			<span class="sources-count text-muted">{sourcesStore.items.length}</span>
			<ChevronRight size={14} />
		</a>
	</section>

	<section>
		<h4>Import</h4>
		{#if !tauri}
			<p class="text-muted section-desc">
				Not available in the web build — install the desktop or Android app to import bookmarks.
			</p>
		{:else}
			<p class="text-muted section-desc">
				Import bookmarks from a Raindrop.io CSV export. Links, tags, and folders are reviewed before capture;
				notes and highlights are not imported.
			</p>
			<div class="row">
				<span class="row-label">Concurrent captures</span>
				<div class="stepper">
					<button
						class="btn btn-icon btn-secondary"
						onclick={() => setImportConcurrency(-1)}
						disabled={settingsStore.current.import_concurrency <= 5}
						aria-label="Fewer concurrent captures"
					>
						–
					</button>
					<span class="stepper-value">{settingsStore.current.import_concurrency}</span>
					<button
						class="btn btn-icon btn-secondary"
						onclick={() => setImportConcurrency(1)}
						disabled={settingsStore.current.import_concurrency >= 10}
						aria-label="More concurrent captures"
					>
						+
					</button>
				</div>
			</div>
			<p class="text-muted section-desc concurrency-desc">
				How many links to capture at once during an import (5–10). Higher finishes a large export
				faster; lower is gentler on the sites you're importing from. Takes effect on the next import.
			</p>
			<button class="btn btn-secondary" onclick={() => uiStore.openImportDialog()}>
				{importStore.running ? `Importing… ${importPct}%` : 'Import from Raindrop'}
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
				<!-- Stays visible — not tied to the dialog — until a new import starts
				     or the user explicitly dismisses it below. Lost on app restart,
				     same as the rest of this in-memory store; that's expected, not a bug. -->
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
		{/if}
	</section>

	<section>
		<h4>Sync</h4>
		<p class="text-muted section-desc">
			Automatically fetch new articles from all sources while Legere is open.
		</p>
		<div class="seg">
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
	</section>

	<section>
		<h4>Updates</h4>
		{#if !tauri}
			<p class="text-muted section-desc">
				Not available in the web build — install the desktop or Android app to get in-app updates.
			</p>
		{:else}
			<div class="row">
				<span class="row-label">Version</span>
				<span class="text-muted">{version || '—'}</span>
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

			<div class="row">
				<span class="row-label">What's new</span>
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

	<section>
		<h4 class="danger-heading">Danger zone</h4>
		<p class="text-muted section-desc">
			Permanently delete every saved article and its files. Sources are kept, but everything
			captured from them is gone — this cannot be undone.
		</p>
		<button class="btn btn-danger" onclick={() => uiStore.openDeleteAllArticlesDialog()}>
			Delete all articles
		</button>
	</section>

	<section>
		<h4>About</h4>
		<p class="text-muted version">Legere — offline article reader.</p>
	</section>
</div>

<style>
	.settings-page {
		max-width: 600px;
		padding: 36px 36px 56px;
	}
	.settings-page h1 {
		font-size: 24px;
		margin: 0 0 24px;
	}
	section {
		margin-bottom: 26px;
	}
	section h4 {
		margin: 0 0 4px;
	}
	.danger-heading {
		color: var(--color-danger);
	}
	.section-desc {
		font-size: 13px;
		margin: 0 0 14px;
	}
	.concurrency-desc {
		margin-top: -4px;
		line-height: 1.5;
	}
	.import-progress {
		margin-top: 12px;
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
		padding: 9px 0;
	}
	.row-label {
		font-size: 14px;
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
		font-variant-numeric: tabular-nums;
	}
	.sources-link {
		display: flex;
		align-items: center;
		gap: 8px;
		padding: 10px 12px;
		margin: 0 -12px;
		border-radius: var(--radius-md);
		text-decoration: none;
		color: var(--color-text);
		font-size: 14px;
	}
	.sources-link:hover {
		background: var(--color-surface);
	}
	.sources-count {
		margin-left: auto;
		font-size: 12px;
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

	@media (max-width: 768px) {
		.settings-page {
			padding: 20px 16px 32px;
		}
	}
</style>
