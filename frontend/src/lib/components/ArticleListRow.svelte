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
			<div class="title-row">
				<h4 class="title">{article.title}</h4>
				{#if article.reading_state === 'unread'}
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
	<button class="delete-btn" onclick={ondelete} aria-label="Delete article">
		<Trash size={14} />
	</button>
</div>

<style>
	.row-wrapper {
		display: flex;
		align-items: center;
		gap: 2px;
		border-radius: var(--radius-md);
	}
	.row-wrapper:hover {
		background: color-mix(in srgb, var(--color-text) 5%, transparent);
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
		border-radius: var(--radius-md);
		cursor: pointer;
		padding: 10px 8px;
		font-family: var(--font-body);
		color: var(--color-text);
		flex: 1;
		min-width: 0;
	}
	.delete-btn {
		flex: none;
		display: flex;
		align-items: center;
		justify-content: center;
		width: 30px;
		height: 30px;
		margin-right: 8px;
		border: none;
		border-radius: var(--radius-sm);
		background: none;
		color: var(--color-neutral-500);
		cursor: pointer;
		transition: opacity 0.1s;
	}
	.delete-btn:hover {
		color: var(--color-accent-200);
		background: color-mix(in srgb, var(--color-text) 8%, transparent);
	}
	.thumb {
		position: relative;
		width: 56px;
		height: 56px;
		flex: none;
		border-radius: var(--radius-sm);
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
