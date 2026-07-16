<script lang="ts">
	import type { ArticleSummary } from '$lib/types';
	import { formatDate, formatReadTime } from '$lib/format';
	import HeroImage from './HeroImage.svelte';

	let { article, onclick }: { article: ArticleSummary; onclick: () => void } = $props();
</script>

<button {onclick} class="article-row">
	<div class="thumb">
		<HeroImage path={article.hero_image_path} alt={article.title} />
	</div>
	<div class="body">
		<div class="title-row">
			<h4 class="title">{article.title}</h4>
			{#if article.unread}
				<span class="unread-dot"></span>
			{/if}
		</div>
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
	.article-row {
		display: flex;
		align-items: center;
		gap: 14px;
		text-align: left;
		background: none;
		border: none;
		border-radius: var(--radius-md);
		cursor: pointer;
		padding: 10px 8px;
		font-family: var(--font-body);
		color: var(--color-text);
		width: 100%;
	}
	.article-row:hover {
		background: color-mix(in srgb, var(--color-text) 5%, transparent);
	}
	.thumb {
		width: 56px;
		height: 56px;
		flex: none;
		border-radius: var(--radius-sm);
		overflow: hidden;
		background: var(--color-surface);
		border: 1px solid var(--color-divider);
	}
	.body {
		flex: 1;
		min-width: 0;
	}
	.title-row {
		display: flex;
		align-items: center;
		gap: 8px;
	}
	.title {
		margin: 0;
		font-size: 15px;
		font-weight: 500;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.unread-dot {
		width: 6px;
		height: 6px;
		border-radius: 50%;
		background: var(--color-accent);
		flex: none;
	}
	.card-meta {
		margin-top: 3px;
	}
</style>
