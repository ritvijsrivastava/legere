<script lang="ts">
	import { page } from '$app/state';
	import { goto } from '$app/navigation';
	import { articlesStore } from '$lib/stores/articles.svelte';
	import { settingsStore } from '$lib/stores/settings.svelte';
	import * as api from '$lib/api';
	import type { ArticleDetail, ReaderMeasure, ReaderLeading } from '$lib/types';
	import HeroImage from '$lib/components/HeroImage.svelte';
	import SegmentedControl from '$lib/components/SegmentedControl.svelte';
	import ReaderControls from '$lib/components/ReaderControls.svelte';
	import ArticleOverflowMenu from '$lib/components/ArticleOverflowMenu.svelte';
	import ChevronLeft from '$lib/icons/ChevronLeft.svelte';
	import Heart from '$lib/icons/Heart.svelte';
	import ExternalLink from '$lib/icons/ExternalLink.svelte';
	import { formatDate } from '$lib/format';

	const SAVE_PROGRESS_DEBOUNCE_MS = 750;
	const MEASURE_PX: Record<ReaderMeasure, number> = { narrow: 600, default: 680, wide: 760 };
	const LEADING_VALUE: Record<ReaderLeading, number> = { compact: 1.6, default: 1.75, airy: 1.9 };

	let article = $state<ArticleDetail | null>(null);
	let view = $state<'readable' | 'original'>('readable');
	let scrollProgress = $state(0);
	let containerEl = $state<HTMLElement | null>(null);
	let scrollParentEl: HTMLElement | null = null;
	let restoredScrollForId: string | null = null;
	let saveProgressTimer: ReturnType<typeof setTimeout> | null = null;

	let fontSize = $state(19);
	let measure = $state<ReaderMeasure>('default');
	let leading = $state<ReaderLeading>('default');
	let initializedReaderSettings = false;

	$effect(() => {
		if (settingsStore.loaded && !initializedReaderSettings) {
			fontSize = settingsStore.current.reader_font_size;
			measure = settingsStore.current.reader_measure;
			leading = settingsStore.current.reader_leading;
			initializedReaderSettings = true;
		}
	});

	$effect(() => {
		const id = page.params.id;
		if (!id) return;
		article = null;
		restoredScrollForId = null;
		api.getArticle(id).then(async (detail) => {
			view = detail.extraction_confident ? 'readable' : 'original';
			if (detail.unread) {
				await articlesStore.markRead(id);
				article = { ...detail, unread: false };
			} else {
				article = detail;
			}
		});
	});

	let resolvedContentHtml = $derived(article ? api.resolveZimTokens(article.content_html) : '');
	let originalUrl = $derived(article ? api.zimUrl(article.id, article.zim_main_path) : '');
	let minutesLeft = $derived(
		article ? Math.max(1, Math.round(article.read_time_min * (1 - scrollProgress))) : 0
	);

	$effect(() => {
		if (!containerEl || view !== 'readable') return;
		const scrollParent = containerEl.closest('.content') as HTMLElement | null;
		if (!scrollParent) return;
		scrollParentEl = scrollParent;

		function onScroll() {
			const max = scrollParent!.scrollHeight - scrollParent!.clientHeight;
			scrollProgress = max > 0 ? Math.min(1, Math.max(0, scrollParent!.scrollTop / max)) : 0;
			scheduleSaveProgress();
		}
		scrollParent.addEventListener('scroll', onScroll);
		return () => scrollParent.removeEventListener('scroll', onScroll);
	});

	// Restores scroll position once per article, after the readable view
	// has rendered — deferred a frame so images/layout have settled and
	// `scrollHeight` reflects the real content height.
	$effect(() => {
		if (!article || !scrollParentEl || view !== 'readable') return;
		if (restoredScrollForId === article.id) return;
		restoredScrollForId = article.id;
		const progress = article.reading_progress;
		if (progress <= 0) {
			scrollParentEl.scrollTop = 0;
			return;
		}
		requestAnimationFrame(() => {
			if (!scrollParentEl) return;
			const max = scrollParentEl.scrollHeight - scrollParentEl.clientHeight;
			scrollParentEl.scrollTop = max > 0 ? progress * max : 0;
			scrollProgress = progress;
		});
	});

	function scheduleSaveProgress() {
		if (!article) return;
		if (saveProgressTimer) clearTimeout(saveProgressTimer);
		const id = article.id;
		const progress = scrollProgress;
		saveProgressTimer = setTimeout(() => {
			api.saveReadingProgress(id, progress);
			const storeItem = articlesStore.items.find((a) => a.id === id);
			if (storeItem) storeItem.reading_progress = progress;
		}, SAVE_PROGRESS_DEBOUNCE_MS);
	}

	function setFontSize(size: number) {
		fontSize = size;
		if (initializedReaderSettings) settingsStore.update({ reader_font_size: size });
	}
	function setMeasure(value: ReaderMeasure) {
		measure = value;
		if (initializedReaderSettings) settingsStore.update({ reader_measure: value });
	}
	function setLeading(value: ReaderLeading) {
		leading = value;
		if (initializedReaderSettings) settingsStore.update({ reader_leading: value });
	}

	async function toggleFavorite() {
		if (!article) return;
		const favorited = await api.toggleFavorite(article.id);
		article = { ...article, favorited };
		const storeItem = articlesStore.items.find((a) => a.id === article!.id);
		if (storeItem) storeItem.favorited = favorited;
	}

	let recapturing = $state(false);

	async function recapture() {
		if (!article) return;
		recapturing = true;
		try {
			article = await api.recaptureArticle(article.id);
			view = article.extraction_confident ? 'readable' : 'original';
		} finally {
			recapturing = false;
		}
	}

	async function deleteArticle() {
		if (!article) return;
		if (!confirm(`Delete "${article.title}"? This can't be undone.`)) return;
		const id = article.id;
		await api.deleteArticle(id);
		articlesStore.items = articlesStore.items.filter((a) => a.id !== id);
		goto('/');
	}
</script>

<div bind:this={containerEl}>
	{#if view === 'readable'}
		<div class="progress-track">
			<div class="progress-fill" style:width="{Math.round(scrollProgress * 100)}%"></div>
		</div>
	{/if}

	<div class="header-row" style:max-width="{MEASURE_PX[measure]}px">
		<button class="btn btn-ghost back-btn" onclick={() => goto('/')}>
			<ChevronLeft />
			Library
		</button>
		{#if article}
			<div class="controls">
				<SegmentedControl
					name="view"
					bind:value={view}
					options={[
						{ value: 'readable', label: 'Readable' },
						{ value: 'original', label: 'Original' }
					]}
				/>
				{#if view === 'readable'}
					<ReaderControls
						{fontSize}
						{measure}
						{leading}
						onFontSize={setFontSize}
						onMeasure={setMeasure}
						onLeading={setLeading}
					/>
				{/if}
				<button
					class="btn btn-icon btn-secondary favorite-btn"
					class:favorited={article.favorited}
					onclick={toggleFavorite}
					aria-label="Favorite"
				>
					<Heart filled={article.favorited} />
				</button>
				<ArticleOverflowMenu {recapturing} onRecapture={recapture} onDelete={deleteArticle} />
			</div>
		{/if}
	</div>

	{#if article}
		{#if view === 'readable'}
			<div class="reader-page" style:max-width="{MEASURE_PX[measure]}px">
				<div class="hero">
					<HeroImage path={article.hero_image_path} alt={article.title} />
				</div>
				<h1 class="reader-title">{article.title}</h1>
				<div class="card-meta reader-meta">
					<span>{article.source_name}</span>
					<span>·</span>
					<span>{formatDate(article.published_at)}</span>
					<span>·</span>
					<span>{minutesLeft} min left</span>
					<a href={article.link} target="_blank" rel="noopener" class="view-original">
						<ExternalLink />
						View original
					</a>
				</div>

				<div
					class="reader-body"
					style:font-size="{fontSize}px"
					style:line-height={LEADING_VALUE[leading]}
				>
					{@html resolvedContentHtml}
				</div>
			</div>
		{:else}
			<iframe class="archive-frame" sandbox="" title={article.title} src={originalUrl}
			></iframe>
		{/if}
	{/if}
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
		margin: 0 auto;
		padding: 0 calc(36px + env(safe-area-inset-right)) 56px calc(36px + env(safe-area-inset-left));
	}
	.header-row {
		display: flex;
		align-items: center;
		justify-content: space-between;
		margin: 0 auto 20px;
		padding: calc(36px + env(safe-area-inset-top)) calc(36px + env(safe-area-inset-right)) 0
			calc(36px + env(safe-area-inset-left));
	}
	.back-btn {
		padding-left: 0;
	}
	.controls {
		display: flex;
		align-items: center;
		gap: 10px;
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
		font-family: var(--font-reading);
		font-variation-settings: 'wght' 600;
		font-size: clamp(28px, 5vw, 38px);
		line-height: 1.15;
		letter-spacing: -0.01em;
		margin: 0 0 12px;
	}
	.reader-meta {
		font-size: 13px;
		margin-bottom: 28px;
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
		font-size: 19px;
	}
	.archive-frame {
		display: block;
		width: 100%;
		height: calc(100vh - 96px);
		border: none;
		background: var(--color-surface);
	}
</style>
