<script lang="ts">
	import { page } from '$app/state';
	import { goto } from '$app/navigation';
	import { libraryStatsStore } from '$lib/stores/libraryStats.svelte';
	import { settingsStore } from '$lib/stores/settings.svelte';
	import * as api from '$lib/api';
	import type { ArticleDetail, ReaderMeasure, ReaderLeading, ReaderTheme } from '$lib/types';
	import HeroImage from '$lib/components/HeroImage.svelte';
	import ReaderControls from '$lib/components/ReaderControls.svelte';
	import TagEditor from '$lib/components/TagEditor.svelte';
	import ArticleOverflowMenu from '$lib/components/ArticleOverflowMenu.svelte';
	import ChevronLeft from '$lib/icons/ChevronLeft.svelte';
	import Star from '$lib/icons/Star.svelte';
	import Check from '$lib/icons/Check.svelte';
	import { formatCompactRelativeTime } from '$lib/format';

	const SAVE_PROGRESS_DEBOUNCE_MS = 750;
	const MEASURE_PX: Record<ReaderMeasure, number> = { narrow: 600, default: 680, wide: 760 };
	const LEADING_VALUE: Record<ReaderLeading, number> = { compact: 1.6, default: 1.75, airy: 1.9 };
	/** `light` tracks the app's own theme (no override, falls through to
	 *  the ambient --color-* tokens); sepia/dark are fixed palettes,
	 *  verbatim from the Legere.dc.html design's `readerThemeMap`. */
	const READER_THEME_OVERRIDES: Record<
		ReaderTheme,
		{ bg: string; fg: string; muted: string; divider: string } | null
	> = {
		light: null,
		sepia: { bg: '#f2e8d8', fg: '#3a2f20', muted: 'rgba(58,47,32,0.6)', divider: 'rgba(58,47,32,0.14)' },
		dark: { bg: '#1a1712', fg: '#ece6d8', muted: 'rgba(236,230,216,0.55)', divider: 'rgba(255,255,255,0.1)' }
	};

	const BACK_LABELS: Record<string, string> = { '/': 'Library', '/favorites': 'Favorites' };
	let backHref = $derived.by(() => {
		const from = page.url.searchParams.get('from');
		return from && from in BACK_LABELS ? from : '/';
	});
	let backLabel = $derived(BACK_LABELS[backHref]);

	let article = $state<ArticleDetail | null>(null);
	let scrollProgress = $state(0);
	let containerEl = $state<HTMLElement | null>(null);
	let scrollParentEl: HTMLElement | null = null;
	let restoredScrollForId: string | null = null;
	let saveProgressTimer: ReturnType<typeof setTimeout> | null = null;

	let fontSize = $state(19);
	let measure = $state<ReaderMeasure>('default');
	let leading = $state<ReaderLeading>('default');
	let readerTheme = $state<ReaderTheme>('light');
	let initializedReaderSettings = false;

	$effect(() => {
		if (settingsStore.loaded && !initializedReaderSettings) {
			fontSize = settingsStore.current.reader_font_size;
			measure = settingsStore.current.reader_measure;
			leading = settingsStore.current.reader_leading;
			readerTheme = settingsStore.current.reader_theme;
			initializedReaderSettings = true;
		}
	});

	let readerPalette = $derived(READER_THEME_OVERRIDES[readerTheme]);

	$effect(() => {
		const id = page.params.id;
		if (!id) return;
		article = null;
		restoredScrollForId = null;
		api.openForReading(id).then((detail) => {
			article = detail;
		});
	});

	let resolvedContentHtml = $derived(article ? api.resolveContentTokens(article.content_html) : '');
	let minutesLeft = $derived(
		article ? Math.max(1, Math.round(article.read_time_min * (1 - scrollProgress))) : 0
	);

	$effect(() => {
		if (!containerEl) return;
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
		if (!article || !scrollParentEl) return;
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
	function setReaderTheme(value: ReaderTheme) {
		readerTheme = value;
		if (initializedReaderSettings) settingsStore.update({ reader_theme: value });
	}

	async function toggleFavorite() {
		if (!article) return;
		const favorited = await api.toggleFavorite(article.id);
		article = { ...article, favorited };
		libraryStatsStore.refresh();
	}

	async function markAsRead() {
		if (!article) return;
		// No explicit `libraryStatsStore.refresh()` here — the backend's
		// `mark_as_read` command already emits `articles:changed`, which
		// the app-wide listener (`events.ts`) turns into one.
		await api.markAsRead(article.id);
		article = { ...article, reading_state: 'read' };
	}

	let recapturing = $state(false);

	async function recapture() {
		if (!article) return;
		recapturing = true;
		try {
			article = await api.recaptureArticle(article.id);
		} finally {
			recapturing = false;
		}
	}

	async function deleteArticle() {
		if (!article) return;
		if (!confirm(`Delete "${article.title}"? This can't be undone.`)) return;
		// No explicit `libraryStatsStore.refresh()` here — `delete_article`
		// already emits `articles:changed` (see the app-wide listener in
		// `events.ts`), and `goto` below remounts the destination view's
		// `ArticleCollection` fresh anyway.
		await api.deleteArticle(article.id);
		goto(backHref);
	}
</script>

<div
	bind:this={containerEl}
	class="reader-root"
	style:background={readerPalette ? readerPalette.bg : 'var(--color-bg)'}
	style:color={readerPalette ? readerPalette.fg : 'var(--color-text)'}
	style:--reader-fg={readerPalette ? readerPalette.fg : 'var(--color-text)'}
	style:--reader-muted={readerPalette ? readerPalette.muted : 'var(--color-muted)'}
	style:--reader-divider={readerPalette ? readerPalette.divider : 'var(--color-divider)'}
>
	<div class="progress-track">
		<div class="progress-fill" style:width="{Math.round(scrollProgress * 100)}%"></div>
	</div>

	<div class="header-row">
		<button class="btn btn-ghost back-btn" onclick={() => goto(backHref)}>
			<ChevronLeft />
			{backLabel}
		</button>
		{#if article}
			<div class="controls">
				<ReaderControls
					{fontSize}
					{measure}
					{leading}
					{readerTheme}
					onFontSize={setFontSize}
					onMeasure={setMeasure}
					onLeading={setLeading}
					onReaderTheme={setReaderTheme}
				/>
				<button
					class="btn btn-icon btn-secondary favorite-btn"
					class:favorited={article.favorited}
					onclick={toggleFavorite}
					aria-label="Favorite"
				>
					<Star filled={article.favorited} />
				</button>
				<button
					class="btn btn-icon btn-secondary read-btn"
					class:read={article.reading_state === 'read'}
					onclick={markAsRead}
					aria-label="Mark as read"
				>
					<Check />
				</button>
				<ArticleOverflowMenu
					{recapturing}
					link={article.link}
					onRecapture={recapture}
					onDelete={deleteArticle}
				/>
			</div>
		{/if}
	</div>

	{#if article}
		<div class="reader-page" style:max-width="{MEASURE_PX[measure]}px">
			{#if article.hero_image_path}
				<div class="hero">
					<HeroImage path={article.hero_image_path} alt={article.title} />
				</div>
			{/if}
			<div class="card-meta reader-meta">
				<span>{article.source_name}</span>
				<span>·</span>
				<span>{minutesLeft} min left</span>
				<span>·</span>
				<span>{formatCompactRelativeTime(article.published_at)}</span>
			</div>
			<h1 class="reader-title">{article.title}</h1>

			<div class="reader-tags">
				<TagEditor
					articleId={article.id}
					tags={article.tags}
					onChange={(tags) => {
						if (article) article = { ...article, tags };
					}}
				/>
			</div>

			<div
				class="reader-body"
				style:font-size="{fontSize}px"
				style:line-height={LEADING_VALUE[leading]}
			>
				{@html resolvedContentHtml}
			</div>
		</div>
	{/if}
</div>

<style>
	.reader-root {
		min-height: 100%;
	}
	.progress-track {
		position: sticky;
		top: 0;
		height: 2px;
		background: var(--reader-divider, var(--color-divider));
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
		flex-wrap: wrap;
		row-gap: 10px;
		margin: 0 0 20px;
		padding: calc(36px + env(safe-area-inset-top)) calc(36px + env(safe-area-inset-right)) 18px
			calc(36px + env(safe-area-inset-left));
		border-bottom: 1px solid var(--reader-divider, var(--color-divider));
	}
	.header-row :global(.btn-secondary) {
		border-color: var(--reader-divider, var(--color-divider));
		color: var(--reader-fg, var(--color-text));
	}
	.header-row :global(.btn-secondary:hover:not(:disabled)) {
		background: color-mix(in srgb, var(--reader-fg, var(--color-text)) 7%, transparent);
	}
	.header-row :global(.seg) {
		border-color: var(--reader-divider, var(--color-divider));
	}
	.header-row :global(.seg-opt) {
		color: var(--reader-fg, var(--color-text));
		border-color: var(--reader-divider, var(--color-divider));
	}
	.header-row :global(.seg-opt:not(:has(input:checked)):hover) {
		background: color-mix(in srgb, var(--reader-fg, var(--color-text)) 7%, transparent);
	}
	.back-btn {
		padding-left: 0;
	}
	.controls {
		display: flex;
		align-items: center;
		flex-wrap: wrap;
		row-gap: 8px;
		gap: 10px;
	}
	.favorite-btn {
		color: var(--reader-fg, var(--color-text));
	}
	.favorite-btn.favorited {
		color: var(--color-accent);
	}
	.read-btn {
		color: var(--reader-fg, var(--color-text));
	}
	.read-btn.read {
		color: var(--color-accent);
	}
	.hero {
		width: 100%;
		aspect-ratio: 21 / 9;
		max-height: 340px;
		margin-bottom: 28px;
		border-radius: var(--radius-lg);
		overflow: hidden;
		background: color-mix(in srgb, var(--reader-fg, var(--color-text)) 6%, transparent);
		border: 1px solid var(--reader-divider, var(--color-divider));
	}
	.reader-meta {
		font-size: 13px;
		margin-bottom: 10px;
		color: var(--reader-muted, var(--color-muted));
	}
	.reader-tags {
		margin: 0 0 28px;
	}
	.reader-title {
		font-family: var(--font-reading);
		font-variation-settings: 'wght' 600;
		font-size: clamp(28px, 5vw, 38px);
		line-height: 1.15;
		letter-spacing: -0.01em;
		margin: 0 0 28px;
	}
	.reader-body {
		font-size: 19px;
	}
</style>
