<script lang="ts">
	import { goto } from '$app/navigation';
	import { libraryStatsStore } from '$lib/stores/libraryStats.svelte';
	import ArticleCollection from '$lib/components/ArticleCollection.svelte';

	function openArticle(id: string) {
		goto(`/reader/${id}?from=/`);
	}
</script>

<ArticleCollection
	title="Library"
	subtitle="{libraryStatsStore.unreadCount} unread"
	emptyMessage="No articles yet. Add a source or paste a direct link to get started."
	showRefresh
	enableTypeSearch
	onopen={openArticle}
/>
