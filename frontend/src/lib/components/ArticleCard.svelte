<script lang="ts">
	import type { ArticleSummary } from '$lib/types';
	import { formatDate, formatReadTime } from '$lib/format';
	import * as api from '$lib/api';
	import { libraryStatsStore } from '$lib/stores/libraryStats.svelte';
	import { sourceDotColor } from '$lib/sourceColor';
	import HeroImage from './HeroImage.svelte';
	import Star from '$lib/icons/Star.svelte';
	import Trash from '$lib/icons/Trash.svelte';
	import FolderMove from '$lib/icons/FolderMove.svelte';

	let {
		article,
		onclick,
		ondelete,
		onmove,
		onMountRoot
	}: {
		article: ArticleSummary;
		onclick: () => void;
		ondelete: () => void;
		onmove: () => void;
		/** Called once this card's root element exists — `ArticleCollection`
		 *  uses it (only on the window's first rendered card) to measure a
		 *  real row height for its virtualized grid. */
		onMountRoot?: (el: HTMLElement) => void;
	} = $props();

	let wrapperEl = $state<HTMLElement | null>(null);
	$effect(() => {
		if (wrapperEl) onMountRoot?.(wrapperEl);
	});

	async function toggleFavorite(e: MouseEvent) {
		e.stopPropagation();
		// Mutating `article` directly (rather than going through a shared
		// store) works because it's the same reactive object the caller's
		// paginated `loadedItems` array holds — see `ArticleCollection`.
		article.favorited = await api.toggleFavorite(article.id);
		libraryStatsStore.refresh();
	}
</script>

<div class="card-wrapper" bind:this={wrapperEl}>
	<div
		{onclick}
		onkeydown={(e) => {
			if (e.target !== e.currentTarget) return;
			if (e.key === 'Enter' || e.key === ' ') {
				e.preventDefault();
				onclick();
			}
		}}
		class="article-card"
		role="button"
		tabindex="0"
	>
		<div class="hero">
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
				<span class="spacer"></span>
				<button
					class="fav-btn"
					class:favorited={article.favorited}
					onclick={toggleFavorite}
					aria-label={article.favorited ? 'Remove from favorites' : 'Add to favorites'}
				>
					<Star size={15} filled={article.favorited} />
				</button>
			</div>
			<div class="title-row">
				<h3 class="card-title">{article.title}</h3>
				{#if article.reading_state === 'unread'}
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
	</div>
	<div class="card-actions">
		<button class="card-action-btn" onclick={onmove} aria-label="Move to category">
			<FolderMove size={13} />
		</button>
		<button class="card-action-btn danger" onclick={ondelete} aria-label="Delete article">
			<Trash size={13} />
		</button>
	</div>
</div>

<style>
	.card-wrapper {
		position: relative;
	}
	.card-wrapper:not(:hover):not(:focus-within) .card-actions {
		opacity: 0;
	}
	.article-card {
		display: flex;
		flex-direction: column;
		width: 100%;
		height: 100%;
		text-align: left;
		background: var(--color-surface);
		border: none;
		border-radius: var(--radius-lg);
		overflow: hidden;
		cursor: pointer;
		padding: 0;
		font-family: var(--font-body);
		color: var(--color-text);
		box-shadow: var(--shadow-card);
		transition: transform var(--duration-base) var(--ease-snap), box-shadow var(--duration-base) var(--ease-snap);
	}
	.article-card:hover {
		transform: translateY(-3px);
		box-shadow: var(--shadow-card-hover);
	}
	.article-card:active {
		transform: translateY(-1px);
	}
	.card-actions {
		position: absolute;
		top: 8px;
		right: 8px;
		z-index: 2;
		display: flex;
		gap: 4px;
		transition: opacity 0.1s;
	}
	.card-action-btn {
		display: flex;
		align-items: center;
		justify-content: center;
		width: 26px;
		height: 26px;
		border: none;
		border-radius: var(--radius-sm);
		background: color-mix(in srgb, black 55%, transparent);
		color: #fff;
		cursor: pointer;
	}
	.card-action-btn:hover {
		background: color-mix(in srgb, black 70%, transparent);
	}
	.card-action-btn.danger:hover {
		color: var(--color-danger);
	}
	.hero {
		position: relative;
		width: 100%;
		/* The hero owns its own proportions instead of the whole card being
		   forced to a 1:1 square — that made cards balloon in height at
		   the fixed grid width, especially with only 1–2 columns fit. */
		aspect-ratio: 16 / 10;
		flex: none;
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
		gap: 8px;
		padding: 14px;
		flex: 1 1 auto;
		min-height: 0;
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
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.spacer {
		flex: 1;
	}
	.fav-btn {
		display: inline-flex;
		flex: none;
		cursor: pointer;
		color: var(--color-muted);
		background: none;
		border: none;
		padding: 0;
		font: inherit;
	}
	.fav-btn.favorited {
		color: var(--color-accent);
	}
	.title-row {
		display: flex;
		align-items: center;
		gap: 8px;
	}
	.title-row .card-title {
		flex: 1;
		margin: 0;
		font-size: 15.5px;
		line-height: 1.3;
		/* Reserve space for 2 lines regardless of actual line count —
		   otherwise a 1-line title makes the whole card shorter than its
		   neighbors, and CSS Grid's row-stretch only pads the invisible
		   wrapper, not this box, leaving visibly mismatched card heights
		   in the same row (see `card-wrapper` sizing above `.article-card`). */
		min-height: calc(1.3em * 2);
		display: -webkit-box;
		-webkit-line-clamp: 2;
		line-clamp: 2;
		-webkit-box-orient: vertical;
		overflow: hidden;
	}
	.card-body {
		flex: 1;
		opacity: 0.75;
		font-size: 12.5px;
		line-height: 1.5;
		/* Same fixed-height reasoning as `.card-title` above. */
		min-height: calc(1.5em * 2);
		display: -webkit-box;
		-webkit-line-clamp: 2;
		line-clamp: 2;
		-webkit-box-orient: vertical;
		overflow: hidden;
	}
	.unread-dot {
		width: 6px;
		height: 6px;
		border-radius: 50%;
		background: var(--color-accent);
		flex: none;
	}
</style>
