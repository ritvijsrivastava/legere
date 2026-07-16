<script lang="ts">
	import type { ArticleSummary } from '$lib/types';
	import { formatDate, formatReadTime } from '$lib/format';
	import HeroImage from './HeroImage.svelte';

	let { article, onclick }: { article: ArticleSummary; onclick: () => void } = $props();
</script>

<button {onclick} class="article-card elev-sm">
	<div class="hero">
		<HeroImage path={article.hero_image_path} alt={article.title} />
	</div>
	<div class="body">
		<div class="title-row">
			<h3 class="card-title">{article.title}</h3>
			{#if article.unread}
				<span class="unread-dot"></span>
			{/if}
		</div>
		<p class="card-body">{article.excerpt}</p>
		<div class="card-meta">
			<span>{article.source_name}</span>
			<span>·</span>
			<span>{formatDate(article.published_at)}</span>
			<span>·</span>
			<span>{formatReadTime(article.read_time_min)}</span>
		</div>
	</div>
</button>

<style>
	.article-card {
		display: flex;
		flex-direction: column;
		text-align: left;
		background: var(--color-surface);
		border: none;
		border-radius: var(--radius-md);
		overflow: hidden;
		cursor: pointer;
		padding: 0;
		font-family: var(--font-body);
		color: var(--color-text);
	}
	.hero {
		width: 100%;
		aspect-ratio: 16 / 9;
		border-bottom: 1px solid var(--color-divider);
	}
	.body {
		display: flex;
		flex-direction: column;
		gap: 6px;
		padding: 14px;
	}
	.title-row {
		display: flex;
		align-items: center;
		gap: 8px;
	}
	.title-row .card-title {
		flex: 1;
		margin: 0;
	}
	.unread-dot {
		width: 6px;
		height: 6px;
		border-radius: 50%;
		background: var(--color-accent);
		flex: none;
	}
</style>
