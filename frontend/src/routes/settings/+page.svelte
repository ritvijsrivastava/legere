<script lang="ts">
	import { settingsStore } from '$lib/stores/settings.svelte';
	import { sourcesStore } from '$lib/stores/sources.svelte';
	import ChevronRight from '$lib/icons/ChevronRight.svelte';
	import type { FontSize, LibraryView, ReaderMeasure, ReaderTheme } from '$lib/types';

	$effect(() => {
		sourcesStore.refresh();
	});

	function setFontSize(size: FontSize) {
		settingsStore.update({ default_font_size: size });
	}
	function setLibraryView(view: LibraryView) {
		settingsStore.update({ default_library_view: view });
	}
	function setAutosync(enabled: boolean) {
		settingsStore.update({ autosync: enabled });
	}
	function setReaderTheme(theme: ReaderTheme) {
		settingsStore.update({ reader_theme: theme });
	}
	function setReaderMeasure(measure: ReaderMeasure) {
		settingsStore.update({ reader_measure: measure });
	}
	function setReaderFontSize(delta: number) {
		const next = Math.max(16, Math.min(22, settingsStore.current.reader_font_size + delta));
		settingsStore.update({ reader_font_size: next });
	}

	const readerThemeOptions: { value: ReaderTheme; label: string }[] = [
		{ value: 'light', label: 'Light' },
		{ value: 'sepia', label: 'Sepia' },
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
		<h4>Reading defaults</h4>
		<p class="text-muted section-desc">Applied the next time you open an article.</p>
		<div class="row">
			<span class="row-label">Theme</span>
			<div class="seg">
				{#each readerThemeOptions as opt (opt.value)}
					<label class="seg-opt">
						<input
							type="radio"
							name="reader-theme2"
							checked={settingsStore.current.reader_theme === opt.value}
							onchange={() => setReaderTheme(opt.value)}
						/>
						<span>{opt.label}</span>
					</label>
				{/each}
			</div>
		</div>
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
		<h4>Reading</h4>
		<p class="text-muted section-desc">Default text size for article view.</p>
		<div class="seg">
			<label class="seg-opt">
				<input
					type="radio"
					name="fs2"
					checked={settingsStore.current.default_font_size === 'small'}
					onchange={() => setFontSize('small')}
				/>
				<span>Small</span>
			</label>
			<label class="seg-opt">
				<input
					type="radio"
					name="fs2"
					checked={settingsStore.current.default_font_size === 'medium'}
					onchange={() => setFontSize('medium')}
				/>
				<span>Medium</span>
			</label>
			<label class="seg-opt">
				<input
					type="radio"
					name="fs2"
					checked={settingsStore.current.default_font_size === 'large'}
					onchange={() => setFontSize('large')}
				/>
				<span>Large</span>
			</label>
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
	.section-desc {
		font-size: 13px;
		margin: 0 0 14px;
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


	@media (max-width: 768px) {
		.settings-page {
			padding: 20px 16px 32px;
		}
	}
</style>
