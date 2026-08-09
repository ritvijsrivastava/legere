<script lang="ts">
	import type { ArticleSummary } from '$lib/types';
	import { formatDate, formatReadTime } from '$lib/format';
	import { articlesStore } from '$lib/stores/articles.svelte';
	import { sourceDotColor } from '$lib/sourceColor';
	import HeroImage from './HeroImage.svelte';
	import Star from '$lib/icons/Star.svelte';
	import Trash from '$lib/icons/Trash.svelte';

	let {
		article,
		onclick,
		ondelete
	}: { article: ArticleSummary; onclick: () => void; ondelete: () => void } = $props();

	function toggleFavorite(e: MouseEvent) {
		e.stopPropagation();
		articlesStore.toggleFavorite(article.id);
	}
</script>

<div class="row-wrapper">
	<button {onclick} class="article-row">
		<div class="thumb">
			<HeroImage path={article.hero_image_path} alt={article.title} />
			{#if article.reading_progress > 0.02 && article.reading_progress < 0.98}
				<div class="progress-track">
					<div class="progress-fill" style:width="{Math.round(article.reading_progress * 100)}%"></div>
				</div>
			{/if}
		</div>
		<div class="body">
			<div class="top-row">
				<span class="dot" style:background={sourceDotColor(article.source_name)}></span>
				<span class="source-label">{article.source_name}</span>
				{#if article.reading_state === 'unread'}
					<span class="unread-dot"></span>
				{/if}
			</div>
			<h4 class="title">{article.title}</h4>
			<div class="card-meta">
				<span>{article.source_name}</span>
				<span>·</span>
				<span>{formatDate(article.published_at)}</span>
				<span>·</span>
				<span>{formatReadTime(article.read_time_min)}</span>
			</div>
		</div>
	</button>
	<button
		class="fav-btn"
		class:favorited={article.favorited}
		onclick={toggleFavorite}
		aria-label={article.favorited ? 'Remove from favorites' : 'Add to favorites'}
	>
		<Star size={15} filled={article.favorited} />
	</button>
	<button class="delete-btn" onclick={ondelete} aria-label="Delete article">
		<Trash size={14} />
	</button>
</div>

<style>
	.row-wrapper {
		display: flex;
		align-items: center;
		gap: 2px;
		border-radius: 10px;
	}
	.row-wrapper:hover {
		background: var(--color-surface);
	}
	.row-wrapper:not(:hover):not(:focus-within) .delete-btn {
		opacity: 0;
	}
	.article-row {
		display: flex;
		align-items: center;
		gap: 14px;
		text-align: left;
		background: none;
		border: none;
		border-radius: 10px;
		cursor: pointer;
		padding: 12px 8px;
		font-family: var(--font-body);
		color: var(--color-text);
		flex: 1;
		min-width: 0;
	}
	.fav-btn,
	.delete-btn {
		flex: none;
		display: flex;
		align-items: center;
		justify-content: center;
		width: 30px;
		height: 30px;
		border: none;
		border-radius: var(--radius-sm);
		background: none;
		color: var(--color-muted);
		cursor: pointer;
		transition: opacity 0.1s;
	}
	.fav-btn.favorited {
		color: var(--color-accent);
	}
	.fav-btn:hover,
	.delete-btn:hover {
		background: color-mix(in srgb, var(--color-text) 8%, transparent);
	}
	.delete-btn {
		margin-right: 8px;
	}
	.delete-btn:hover {
		color: var(--color-danger);
	}
	.thumb {
		position: relative;
		width: 52px;
		height: 52px;
		flex: none;
		border-radius: 10px;
		overflow: hidden;
		background: var(--color-surface);
		border: 1px solid var(--color-divider);
	}
	.progress-track {
		position: absolute;
		left: 0;
		right: 0;
		bottom: 0;
		height: 2px;
		background: color-mix(in srgb, black 45%, transparent);
	}
	.progress-fill {
		height: 100%;
		background: var(--color-accent);
	}
	.body {
		flex: 1;
		min-width: 0;
		display: flex;
		flex-direction: column;
		gap: 3px;
	}
	.top-row {
		display: flex;
		align-items: center;
		gap: 6px;
	}
	.dot {
		width: 7px;
		height: 7px;
		border-radius: 50%;
		flex: none;
	}
	.source-label {
		font-size: 11px;
		color: var(--color-muted);
	}
	.title {
		margin: 0;
		font-family: var(--font-heading);
		font-size: 14.5px;
		font-weight: 600;
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
</style>
