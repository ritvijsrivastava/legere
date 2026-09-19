<script lang="ts">
	import '../app.css';
	import favicon from '$lib/assets/favicon.svg';
	import Shell from '$lib/components/Shell.svelte';
	import AddSourceDialog from '$lib/components/AddSourceDialog.svelte';
	import CaptureJobsPanel from '$lib/components/CaptureJobsPanel.svelte';
	import ImportRaindropDialog from '$lib/components/ImportRaindropDialog.svelte';
	import ImportArticlesDialog from '$lib/components/ImportArticlesDialog.svelte';
	import ImportSourcesDialog from '$lib/components/ImportSourcesDialog.svelte';
	import DeleteAllArticlesDialog from '$lib/components/DeleteAllArticlesDialog.svelte';
	import MoveToCategoryDialog from '$lib/components/MoveToCategoryDialog.svelte';
	import Toast from '$lib/components/Toast.svelte';
	import { libraryStatsStore } from '$lib/stores/libraryStats.svelte';
	import { settingsStore } from '$lib/stores/settings.svelte';
	import { captureJobsStore } from '$lib/stores/captureJobs.svelte';
	import { registerBackendEvents } from '$lib/events';

	let { children } = $props();

	$effect(() => {
		libraryStatsStore.refresh();
		settingsStore.refresh();
		captureJobsStore.refresh();
		registerBackendEvents();
	});

	$effect(() => {
		document.documentElement.dataset.theme = settingsStore.current.app_theme;
	});
</script>

<svelte:head>
	<link rel="icon" href={favicon} />
</svelte:head>

<Shell>
	{@render children()}
</Shell>
<AddSourceDialog />
<CaptureJobsPanel />
<ImportRaindropDialog />
<ImportArticlesDialog />
<ImportSourcesDialog />
<DeleteAllArticlesDialog />
<MoveToCategoryDialog />
<Toast />
