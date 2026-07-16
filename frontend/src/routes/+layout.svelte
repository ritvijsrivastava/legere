<script lang="ts">
	import '../app.css';
	import favicon from '$lib/assets/favicon.svg';
	import Shell from '$lib/components/Shell.svelte';
	import AddSourceDialog from '$lib/components/AddSourceDialog.svelte';
	import { articlesStore } from '$lib/stores/articles.svelte';
	import { settingsStore } from '$lib/stores/settings.svelte';

	let { children } = $props();

	$effect(() => {
		articlesStore.refresh();
		settingsStore.refresh();
	});
</script>

<svelte:head>
	<link rel="icon" href={favicon} />
</svelte:head>

<Shell>
	{@render children()}
</Shell>
<AddSourceDialog />
