<script lang="ts">
	import { page } from '$app/state';
	import { goto } from '$app/navigation';
	import { articlesStore } from '$lib/stores/articles.svelte';
	import { settingsStore } from '$lib/stores/settings.svelte';
	import * as api from '$lib/api';
	import type { ArticleDetail, FontSize } from '$lib/types';
	import HeroImage from '$lib/components/HeroImage.svelte';
	import ChevronLeft from '$lib/icons/ChevronLeft.svelte';
	import Heart from '$lib/icons/Heart.svelte';
	import ExternalLink from '$lib/icons/ExternalLink.svelte';
	import { formatDate, formatReadTime } from '$lib/format';

	let article = $state<ArticleDetail | null>(null);
	let fontSize = $state<FontSize>('medium');
	let scrollProgress = $state(0);
	let containerEl = $state<HTMLElement | null>(null);
	let initializedFontSize = false;

	const fontSizePx: Record<FontSize, number> = { small: 16, medium: 18, large: 21 };

	$effect(() => {
		if (settingsStore.loaded && !initializedFontSize) {
			fontSize = settingsStore.current.default_font_size;
			initializedFontSize = true;
		}
	});

	$effect(() => {
		const id = page.params.id;
		if (!id) return;
		article = null;
		api.getArticle(id).then(async (detail) => {
			if (detail.unread) {
				await articlesStore.markRead(id);
				article = { ...detail, unread: false };
			} else {
				article = detail;
			}
		});
	});

	$effect(() => {
		if (!containerEl) return;
		const scrollParent = containerEl.closest('.content') as HTMLElement | null;
		if (!scrollParent) return;
		scrollParent.scrollTop = 0;
		function onScroll() {
			const max = scrollParent!.scrollHeight - scrollParent!.clientHeight;
			scrollProgress = max > 0 ? Math.min(1, Math.max(0, scrollParent!.scrollTop / max)) : 0;
		}
		scrollParent.addEventListener('scroll', onScroll);
		return () => scrollParent.removeEventListener('scroll', onScroll);
	});

	function setFontSize(size: FontSize) {
		fontSize = size;
		if (initializedFontSize) settingsStore.update({ default_font_size: size });
	}

	async function toggleFavorite() {
		if (!article) return;
		const favorited = await api.toggleFavorite(article.id);
		article = { ...article, favorited };
		const storeItem = articlesStore.items.find((a) => a.id === article!.id);
		if (storeItem) storeItem.favorited = favorited;
	}
</script>

<div bind:this={containerEl}>
	<div class="progress-track">
		<div class="progress-fill" style:width="{Math.round(scrollProgress * 100)}%"></div>
	</div>

	<div class="reader-page">
		<div class="header-row">
			<button class="btn btn-ghost back-btn" onclick={() => goto('/')}>
				<ChevronLeft />
				Library
			</button>
			<div class="controls">
				<div class="seg">
					<label class="seg-opt">
						<input type="radio" name="fs" checked={fontSize === 'small'} onchange={() => setFontSize('small')} />
						<span>S</span>
					</label>
					<label class="seg-opt">
						<input type="radio" name="fs" checked={fontSize === 'medium'} onchange={() => setFontSize('medium')} />
						<span>M</span>
					</label>
					<label class="seg-opt">
						<input type="radio" name="fs" checked={fontSize === 'large'} onchange={() => setFontSize('large')} />
						<span>L</span>
					</label>
				</div>
				{#if article}
					<button
						class="btn btn-icon btn-secondary favorite-btn"
						class:favorited={article.favorited}
						onclick={toggleFavorite}
						aria-label="Favorite"
					>
						<Heart filled={article.favorited} />
					</button>
				{/if}
			</div>
		</div>

		{#if article}
			<div class="hero">
				<HeroImage path={article.hero_image_path} alt={article.title} />
			</div>
			<h1 class="reader-title">{article.title}</h1>
			<div class="card-meta reader-meta">
				<span>{article.source_name}</span>
				<span>·</span>
				<span>{formatDate(article.published_at)}</span>
				<span>·</span>
				<span>{formatReadTime(article.read_time_min)}</span>
				<a href={article.link} target="_blank" rel="noopener" class="view-original">
					<ExternalLink />
					View original
				</a>
			</div>
			<div class="reader-body" style:font-size="{fontSizePx[fontSize]}px">
				{@html article.content_html}
			</div>
		{/if}
	</div>
</div>

<style>
	.progress-track {
		position: sticky;
		top: 0;
		height: 2px;
		background: var(--color-divider);
		z-index: 4;
	}
	.progress-fill {
		height: 100%;
		background: var(--color-accent);
	}
	.reader-page {
		max-width: 660px;
		margin: 0 auto;
		padding: 36px 36px 56px;
	}
	.header-row {
		display: flex;
		align-items: center;
		justify-content: space-between;
		margin-bottom: 20px;
	}
	.back-btn {
		padding-left: 0;
	}
	.controls {
		display: flex;
		align-items: center;
		gap: 6px;
	}
	.favorite-btn {
		color: var(--color-text);
	}
	.favorite-btn.favorited {
		color: var(--color-accent);
	}
	.hero {
		width: 100%;
		aspect-ratio: 16 / 9;
		margin-bottom: 24px;
		border-radius: var(--radius-md);
		overflow: hidden;
		background: var(--color-surface);
		border: 1px solid var(--color-divider);
	}
	.reader-title {
		font-size: 34px;
		font-weight: 500;
		margin: 0 0 12px;
	}
	.reader-meta {
		font-size: 13px;
		margin-bottom: 20px;
	}
	.view-original {
		display: inline-flex;
		align-items: center;
		gap: 5px;
		margin-left: 8px;
		color: var(--color-neutral-500);
		text-decoration: none;
	}
	.reader-body {
		line-height: 1.7;
		color: var(--color-neutral-200);
	}
	.reader-body :global(p) {
		margin: 0 0 20px;
	}
</style>
