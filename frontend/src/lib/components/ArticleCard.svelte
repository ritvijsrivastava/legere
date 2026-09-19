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
		// Blur immediately — otherwise the button keeps focus after the
		// click, which keeps `.card-wrapper:focus-within` true and the
		// hover-only `.card-actions` pinned visible until focus moves to
		// another card or a blank area, reading as the card staying
		// "selected".
		(e.currentTarget as HTMLElement).blur();
		// Mutating `article` directly (rather than going through a shared
		// store) works because it's the same reactive object the caller's
		// paginated `loadedItems` array holds — see `ArticleCollection`.
		article.favorited = await api.toggleFavorite(article.id);
		libraryStatsStore.refresh();
		onfavorite?.();
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
				<CategoryIcon icon={article.category_icon} size={12} />
				<span class="source-label">{article.category_name ?? 'Uncategorized'}</span>
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
			</div>
			<p class="card-body">{article.excerpt}</p>
			<div class="card-meta">
				<span>{formatHost(article.link)}</span>
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
	/* Touch can't truly "hover" — on a touchscreen this can only fire on
	   tap and then stick until something else is tapped, reading as a
	   glitch (card stays lifted/shadowed after the finger's gone) rather
	   than a hover affordance. Gated to real pointer-hover devices. */
	@media (hover: hover) and (pointer: fine) {
		.article-card:hover {
			transform: translateY(-3px);
			box-shadow: var(--shadow-card-hover);
		}
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
	/* `:hover`/`:focus-within` (see `.card-wrapper` above) can't be
	   produced by a touchscreen, and at this card's mobile size (half a
	   152–208px-wide two-up grid cell) a pair of always-visible icon
	   buttons would be too small a target to hit reliably anyway — so
	   mobile cards drop Move/Delete entirely rather than force either a
	   hidden-until-hover control or an undersized touch target. Both
	   actions are still one tap away inside the reader (see the matching
	   comment in `ArticleListRow`, which drops them from the mobile list
	   row for the same reason). */
	@media (max-width: 768px) {
		.card-actions {
			display: none;
		}
	}
	@media (hover: hover) and (pointer: fine) {
		.card-action-btn:hover {
			background: color-mix(in srgb, black 70%, transparent);
		}
		.card-action-btn.danger:hover {
			color: var(--color-danger);
		}
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
	/* The mobile 2-up grid runs cards at ~150–210px instead of desktop's
	   216–264px — a touch tighter so type/padding stay proportional
	   rather than looking like a shrunk desktop card. */
	@media (max-width: 768px) {
		.body {
			gap: 6px;
			padding: 10px;
		}
		.title-row .card-title {
			font-size: 14px;
		}
		.card-body {
			font-size: 12px;
		}
	}
	.top-row {
		display: flex;
		align-items: center;
		gap: 6px;
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
	}
	.title-row .card-title {
		flex: 1;
		margin: 0;
		font-size: 15.5px;
		line-height: 1.3;
		/* No reserved 2-line min-height here (a 1-line title used to leave a
		   dead gap above the excerpt) — `.card-body` below is the flexible
		   one (`flex: 1`), so when CSS Grid's row-stretch makes a short card
		   match its row's tallest neighbor, the slack lands there instead,
		   which reads as generous excerpt spacing rather than a gap wedged
		   between headline and body text. */
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
		/* Same fixed-height reasoning as `.card-title` above — 3 lines here
		   (vs. the title's 2) since the excerpt is the one place on the card
		   with room to actually say something. `max-height` matches
		   `min-height` — without it, a card stretched taller than its
		   neighbor (CSS Grid row-stretch, see `card-wrapper`) lets this
		   `flex: 1` box grow past its natural 3-line height, and
		   `-webkit-line-clamp` on an over-grown flex item stops clamping to
		   a line *count* and just reveals however much text fits the extra
		   height instead. Capping it keeps the clamp exact regardless of
		   row-stretch; the leftover space lands below `.card-meta` instead
		   (pinned there via its own `margin-top: auto`, below). */
		min-height: calc(1.5em * 3);
		max-height: calc(1.5em * 3);
		display: -webkit-box;
		-webkit-line-clamp: 3;
		line-clamp: 3;
		-webkit-box-orient: vertical;
		overflow: hidden;
	}
	.card-meta {
		/* Pins to the bottom of `.body`'s flex column — now that
		   `.card-body` above has a hard `max-height`, it's the only
		   remaining sink for a row-stretched card's extra height, so a
		   short excerpt reads as breathing room above the footer rather
		   than a gap wedged between the excerpt and its own text. */
		margin-top: auto;
	}
</style>
