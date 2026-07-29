<script lang="ts">
	import type { ArticleSummary } from '$lib/types';
	import { formatDate, formatReadTime } from '$lib/format';
	import HeroImage from './HeroImage.svelte';
	import Trash from '$lib/icons/Trash.svelte';

	let {
		article,
		onclick,
		ondelete
	}: { article: ArticleSummary; onclick: () => void; ondelete: () => void } = $props();
</script>

<div class="card-wrapper">
	<button {onclick} class="article-card elev-sm">
		<div class="hero">
			<HeroImage path={article.hero_image_path} alt={article.title} />
			{#if article.reading_progress > 0.02 && article.reading_progress < 0.98}
				<div class="progress-track">
					<div class="progress-fill" style:width="{Math.round(article.reading_progress * 100)}%"></div>
				</div>
			{/if}
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
	<button class="delete-btn" onclick={ondelete} aria-label="Delete article">
		<Trash size={13} />
	</button>
</div>

<style>
	.card-wrapper {
		position: relative;
	}
	.card-wrapper:not(:hover):not(:focus-within) .delete-btn {
		opacity: 0;
	}
	.article-card {
		display: flex;
		flex-direction: column;
		width: 100%;
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
	.delete-btn {
		position: absolute;
		top: 8px;
		right: 8px;
		z-index: 2;
		display: flex;
		align-items: center;
		justify-content: center;
		width: 26px;
		height: 26px;
		border: none;
		border-radius: var(--radius-sm);
		background: color-mix(in srgb, black 55%, transparent);
		color: var(--color-neutral-100);
		cursor: pointer;
		transition: opacity 0.1s;
	}
	.delete-btn:hover {
		background: color-mix(in srgb, black 70%, transparent);
		color: var(--color-accent-200);
	}
	.hero {
		position: relative;
		width: 100%;
		aspect-ratio: 16 / 9;
		border-bottom: 1px solid var(--color-divider);
	}
	.progress-track {
		position: absolute;
		left: 0;
		right: 0;
		bottom: 0;
		height: 3px;
		background: color-mix(in srgb, black 45%, transparent);
	}
	.progress-fill {
		height: 100%;
		background: var(--color-accent);
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
