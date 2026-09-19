<script lang="ts">
	import type { ArticleSummary } from '$lib/types';
	import { formatDate, formatHost, formatReadTime } from '$lib/format';
	import * as api from '$lib/api';
	import { libraryStatsStore } from '$lib/stores/libraryStats.svelte';
	import CategoryIcon from './CategoryIcon.svelte';
	import HeroImage from './HeroImage.svelte';
	import Star from '$lib/icons/Star.svelte';
	import Trash from '$lib/icons/Trash.svelte';
	import FolderMove from '$lib/icons/FolderMove.svelte';

	let {
		article,
		onclick,
		ondelete,
		onmove,
		onfavorite,
		onMountRoot
	}: {
		article: ArticleSummary;
		onclick: () => void;
		ondelete: () => void;
		onmove: () => void;
		onfavorite?: () => void;
		/** Called once this row's root element exists — `ArticleCollection`
		 *  uses it (only on the window's first rendered row) to measure a
		 *  real row height for its virtualized list. */
		onMountRoot?: (el: HTMLElement) => void;
	} = $props();

	let wrapperEl = $state<HTMLElement | null>(null);
	$effect(() => {
		if (wrapperEl) onMountRoot?.(wrapperEl);
	});

	async function toggleFavorite(e: MouseEvent) {
		e.stopPropagation();
		// Blur immediately — see the matching comment in `ArticleCard`.
		(e.currentTarget as HTMLElement).blur();
		// Mutating `article` directly (rather than going through a shared
		// store) works because it's the same reactive object the caller's
		// paginated `loadedItems` array holds — see `ArticleCollection`.
		article.favorited = await api.toggleFavorite(article.id);
		libraryStatsStore.refresh();
		onfavorite?.();
	}
</script>

<div class="row-wrapper" bind:this={wrapperEl}>
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
				<CategoryIcon icon={article.category_icon} size={12} />
				<span class="source-label">{article.category_name ?? 'Uncategorized'}</span>
			</div>
			<h4 class="title">{article.title}</h4>
			<p class="excerpt">{article.excerpt}</p>
			<div class="card-meta">
				<span>{formatHost(article.link)}</span>
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
	<button class="move-btn" onclick={onmove} aria-label="Move to category">
		<FolderMove size={14} />
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
	@media (hover: hover) and (pointer: fine) {
		.row-wrapper:hover {
			background: var(--color-surface);
		}
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
		/* Extra right padding (vs. 8px on every other edge) so a full-width
		   title ending in an ellipsis doesn't sit flush against the
		   favorite star — `.row-wrapper`'s own 2px gap alone read as no gap
		   at all once the title itself ran edge-to-edge. */
		padding: 12px 16px 12px 8px;
		font-family: var(--font-body);
		color: var(--color-text);
		flex: 1;
		min-width: 0;
	}
	.fav-btn,
	.move-btn,
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
	}
	.fav-btn.favorited {
		color: var(--color-accent);
	}
	/* Move/Delete collapse to zero width by default (not just faded via
	   `opacity`) so a favorite-only row never reserves invisible trailing
	   space for them — a fixed 30px-wide-but-`opacity:0` button still
	   pushed `.fav-btn` away from the row's right edge on every device
	   that doesn't happen to match a `:hover` condition, which in practice
	   meant any touch device (touch can't produce `:hover` at all). These
	   rules must come after the shared `width: 30px` block above — same
	   specificity, so source order decides the winner. Expanding them back
	   on hover is a fine-pointer-only affordance (see below); everywhere
	   else they simply stay collapsed, `.fav-btn` flush against the row's
	   right edge. */
	.move-btn,
	.delete-btn {
		width: 0;
		margin-right: 0;
		padding: 0;
		opacity: 0;
		overflow: hidden;
		transition: width var(--duration-fast) var(--ease-snap), opacity var(--duration-fast) var(--ease-snap);
	}
	/* Touch: Move and Delete are one tap away inside the reader (its header
	   already has both, see `routes/reader/[id]/+page.svelte`) and can
	   never expand via `:hover` there anyway (see `ArticleCard`'s matching
	   comment) — the collapse above already keeps the row correct with no
	   breakpoint needed. This only enlarges the one action touch keeps,
	   Favorite, to a real touch target. */
	@media (pointer: coarse) {
		.fav-btn {
			width: 36px;
			height: 36px;
		}
	}
	@media (hover: hover) and (pointer: fine) {
		.fav-btn:hover,
		.move-btn:hover,
		.delete-btn:hover {
			background: color-mix(in srgb, var(--color-text) 8%, transparent);
		}
		.delete-btn:hover {
			color: var(--color-danger);
		}
		.row-wrapper:hover .move-btn,
		.row-wrapper:hover .delete-btn,
		.row-wrapper:focus-within .move-btn,
		.row-wrapper:focus-within .delete-btn {
			width: 30px;
			opacity: 1;
		}
		.row-wrapper:hover .delete-btn,
		.row-wrapper:focus-within .delete-btn {
			margin-right: 8px;
		}
	}
	.thumb {
		position: relative;
		/* Sized to match the row's new content height now that `.excerpt`
		   adds two lines (see below) — the old 52px thumb read as an
		   undersized afterthought once the text column grew past it. */
		width: 76px;
		height: 76px;
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
	.excerpt {
		margin: 0;
		font-size: 12.5px;
		line-height: 1.4;
		color: var(--color-muted);
		opacity: 0.85;
		display: -webkit-box;
		-webkit-line-clamp: 2;
		line-clamp: 2;
		-webkit-box-orient: vertical;
		overflow: hidden;
	}
</style>
