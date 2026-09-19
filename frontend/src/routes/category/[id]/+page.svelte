<script lang="ts">
	import { page } from '$app/state';
	import { goto } from '$app/navigation';
	import { libraryFiltersStore } from '$lib/stores/libraryFilters.svelte';
	import { libraryStatsStore } from '$lib/stores/libraryStats.svelte';
	import ArticleCollection from '$lib/components/ArticleCollection.svelte';
	import CategorySettingsDialog from '$lib/components/CategorySettingsDialog.svelte';
	import SettingsIcon from '$lib/icons/Settings.svelte';

	const UNCATEGORIZED_ID = '__uncategorized__';

	let categoryId = $derived(page.params.id ?? '');
	let isUncategorized = $derived(categoryId === UNCATEGORIZED_ID);

	// The sidebar's aggregate list already carries every real category
	// (with a live count) plus the virtual Uncategorized entry \u2014 reused
	// here instead of a separate fetch so the title/count always matches
	// what was just clicked, and stays live as articles move in/out.
	let category = $derived(libraryStatsStore.categories.find((c) => c.id === categoryId) ?? null);

	// Scope every fetch in `ArticleCollection` to this one category for as
	// long as this page is mounted; restore the unfiltered state on the
	// way out so navigating to Library/Favorites afterward isn't silently
	// still filtered.
	$effect(() => {
		libraryFiltersStore.categoryId = categoryId || null;
		return () => {
			libraryFiltersStore.categoryId = null;
		};
	});

	function openArticle(id: string) {
		goto(`/reader/${id}?from=/category/${categoryId}`);
	}

	let settingsOpen = $state(false);

	function handleDeleted() {
		settingsOpen = false;
		goto('/');
	}
</script>

{#if category}
	<ArticleCollection
		title={category.name}
		subtitle="{category.article_count} article{category.article_count === 1 ? '' : 's'}"
		emptyMessage={isUncategorized
			? 'No uncategorized articles \u2014 everything has a home.'
			: `No articles in "${category.name}" yet. Move one here from its card or the reader.`}
		hideCategoryChips
		onopen={openArticle}
	>
		{#snippet headerActions()}
			{#if !isUncategorized}
				<button
					class="btn btn-icon btn-secondary"
					onclick={() => (settingsOpen = true)}
					aria-label="Category settings"
				>
					<SettingsIcon size={15} />
				</button>
			{/if}
		{/snippet}
	</ArticleCollection>
	{#if !isUncategorized}
		<CategorySettingsDialog
			open={settingsOpen}
			{category}
			onclose={() => (settingsOpen = false)}
			onDeleted={handleDeleted}
		/>
	{/if}
{/if}
