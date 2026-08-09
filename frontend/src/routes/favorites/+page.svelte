<script lang="ts">
	import { goto } from '$app/navigation';
	import { articlesStore } from '$lib/stores/articles.svelte';
	import ArticleCollection from '$lib/components/ArticleCollection.svelte';
	import * as api from '$lib/api';
	import type { ArticleSummary } from '$lib/types';

	let favorited = $derived(articlesStore.items.filter((a) => a.favorited));

	function openArticle(id: string) {
		goto(`/reader/${id}?from=/favorites`);
	}

	async function deleteArticle(article: ArticleSummary) {
		if (!confirm(`Delete "${article.title}"? This can't be undone.`)) return;
		await api.deleteArticle(article.id);
		articlesStore.items = articlesStore.items.filter((a) => a.id !== article.id);
	}
</script>

<ArticleCollection
	title="Favorites"
	items={favorited}
	emptyMessage="No favorites yet. Tap the star on an article to save it here."
	onopen={openArticle}
	ondelete={deleteArticle}
/>
