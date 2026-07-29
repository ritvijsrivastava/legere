<script lang="ts">
	import '../app.css';
	import favicon from '$lib/assets/favicon.svg';
	import Shell from '$lib/components/Shell.svelte';
	import AddSourceDialog from '$lib/components/AddSourceDialog.svelte';
	import Toast from '$lib/components/Toast.svelte';
	import { articlesStore } from '$lib/stores/articles.svelte';
	import { settingsStore } from '$lib/stores/settings.svelte';
	import { registerBackendEvents } from '$lib/events';

	let { children } = $props();

	$effect(() => {
		articlesStore.refresh();
		settingsStore.refresh();
		registerBackendEvents();
	});
</script>

<svelte:head>
	<link rel="icon" href={favicon} />
</svelte:head>

<Shell>
	{@render children()}
</Shell>
<AddSourceDialog />
<Toast />
