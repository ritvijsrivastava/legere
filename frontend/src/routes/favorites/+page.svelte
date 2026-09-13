<script lang="ts">
	import { goto } from '$app/navigation';
	import ArticleCollection from '$lib/components/ArticleCollection.svelte';

	function openArticle(id: string) {
		goto(`/reader/${id}?from=/favorites`);
	}
</script>

<ArticleCollection
	title="Favorites"
	favoritedOnly
	emptyMessage="No favorites yet. Tap the star on an article to save it here."
	onopen={openArticle}
/>
