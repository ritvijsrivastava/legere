<script lang="ts">
	import { settingsStore } from '$lib/stores/settings.svelte';
	import type { FontSize, LibraryView } from '$lib/types';

	function setFontSize(size: FontSize) {
		settingsStore.update({ default_font_size: size });
	}
	function setLibraryView(view: LibraryView) {
		settingsStore.update({ default_library_view: view });
	}
	function setAutosync(enabled: boolean) {
		settingsStore.update({ autosync: enabled });
	}
</script>

<div class="settings-page">
	<h1>Settings</h1>

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

	<p class="version">Legere · Version 0.1.0</p>
</div>

<style>
	.settings-page {
		max-width: 600px;
		margin: 0 auto;
		padding: 36px 36px 56px;
	}
	.settings-page h1 {
		margin: 0 0 24px;
		font-size: 30px;
		font-weight: 500;
	}
	section {
		margin-bottom: 26px;
	}
	section h4 {
		margin: 0 0 4px;
	}
	.section-desc {
		font-size: 13px;
		margin: 0 0 12px;
	}
	.version {
		font-size: 12px;
		color: var(--color-neutral-600);
		margin: 32px 0 0;
	}
</style>
