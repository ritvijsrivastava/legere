<script lang="ts">
	import { page } from '$app/state';
	import { goto } from '$app/navigation';
	import { libraryStatsStore } from '$lib/stores/libraryStats.svelte';
	import { settingsStore } from '$lib/stores/settings.svelte';
	import * as api from '$lib/api';
	import type { AppTheme, ArticleDetail, ReaderMeasure, ReaderLeading, ReadingOverrides } from '$lib/types';
	import HeroImage from '$lib/components/HeroImage.svelte';
	import ReaderControls from '$lib/components/ReaderControls.svelte';
	import TagEditor from '$lib/components/TagEditor.svelte';
	import ArticleOverflowMenu from '$lib/components/ArticleOverflowMenu.svelte';
	import { uiStore } from '$lib/stores/ui.svelte';
	import { openExternalUrl, shareArticleLink } from '$lib/articleActions';
	import ChevronLeft from '$lib/icons/ChevronLeft.svelte';
	import ExternalLink from '$lib/icons/ExternalLink.svelte';
	import Share from '$lib/icons/Share.svelte';
	import Star from '$lib/icons/Star.svelte';
	import { formatCompactRelativeTime, formatReadTime } from '$lib/format';

	const SAVE_PROGRESS_DEBOUNCE_MS = 750;
	const MEASURE_PX: Record<ReaderMeasure, number> = { narrow: 600, default: 680, wide: 760 };
	const LEADING_VALUE: Record<ReaderLeading, number> = { compact: 1.6, default: 1.75, airy: 1.9 };

	const BACK_LABELS: Record<string, string> = { '/': 'Library', '/favorites': 'Favorites' };
	let backHref = $derived.by(() => {
		const from = page.url.searchParams.get('from');
		if (from && (from in BACK_LABELS || from.startsWith('/category/'))) return from;
		return '/';
	});
	// `/category/<id>` isn't in the static map — its label is the
	// category's live name, looked up from the sidebar's aggregate list
	// (already fetched for the whole app; see `libraryStatsStore`).
	let backLabel = $derived.by(() => {
		if (backHref in BACK_LABELS) return BACK_LABELS[backHref];
		if (backHref.startsWith('/category/')) {
			const id = backHref.slice('/category/'.length);
			const category = libraryStatsStore.categories.find((c) => c.id === id);
			return category?.name ?? 'Category';
		}
		return 'Library';
	});

	let article = $state<ArticleDetail | null>(null);
	let scrollProgress = $state(0);
	let containerEl = $state<HTMLElement | null>(null);
	let scrollParentEl: HTMLElement | null = null;
	let restoredScrollForId: string | null = null;
	let saveProgressTimer: ReturnType<typeof setTimeout> | null = null;

	let fontSize = $state(19);
	let measure = $state<ReaderMeasure>('default');
	let leading = $state<ReaderLeading>('default');
	let theme = $state<AppTheme>('dark');
	// Tracks which article's overrides are currently loaded into the four
	// `$state` values above, so the effect below re-derives them exactly
	// once per article (not once per whole session, and not on every
	// unrelated `article` reassignment within the same article) — each
	// article can carry its own overrides (`ReadingOverrides`), falling
	// back field-by-field to the global `settingsStore` default.
	let overridesLoadedForId: string | null = null;

	$effect(() => {
		if (!article || !settingsStore.loaded) return;
		if (overridesLoadedForId === article.id) return;
		overridesLoadedForId = article.id;
		fontSize = article.overrides.font_size ?? settingsStore.current.reader_font_size;
		measure = article.overrides.measure ?? settingsStore.current.reader_measure;
		leading = article.overrides.leading ?? settingsStore.current.reader_leading;
		theme = article.overrides.theme ?? settingsStore.current.app_theme;
	});

	let hasOverride = $derived(
		article !== null &&
			(article.overrides.font_size !== null ||
				article.overrides.measure !== null ||
				article.overrides.leading !== null ||
				article.overrides.theme !== null)
	);

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

	// Persists one changed field of this article's overrides, keeping the
	// other three whatever they already were (an article previously
	// overriding only its theme, say, doesn't lose that when its font size
	// is then changed too) — unlike the old behavior, none of this ever
	// touches the global `settingsStore`.
	function persistOverrides(next: Partial<ReadingOverrides>) {
		if (!article) return;
		const overrides = { ...article.overrides, ...next };
		article = { ...article, overrides };
		api.setReadingOverrides(article.id, overrides);
	}
	function setFontSize(size: number) {
		fontSize = size;
		persistOverrides({ font_size: size });
	}
	function setMeasure(value: ReaderMeasure) {
		measure = value;
		persistOverrides({ measure: value });
	}
	function setLeading(value: ReaderLeading) {
		leading = value;
		persistOverrides({ leading: value });
	}
	function setTheme(value: AppTheme) {
		theme = value;
		persistOverrides({ theme: value });
	}
	/** Clears every override on the current article, falling back to
	 *  whatever the global settings are right now. */
	function resetOverrides() {
		if (!article) return;
		const overrides: ReadingOverrides = {
			font_size: null,
			measure: null,
			leading: null,
			theme: null
		};
		article = { ...article, overrides };
		api.setReadingOverrides(article.id, overrides);
		fontSize = settingsStore.current.reader_font_size;
		measure = settingsStore.current.reader_measure;
		leading = settingsStore.current.reader_leading;
		theme = settingsStore.current.app_theme;
	}

	async function toggleFavorite() {
		if (!article) return;
		const favorited = await api.toggleFavorite(article.id);
		article = { ...article, favorited };
		libraryStatsStore.refresh();
	}

	async function openOriginal() {
		if (!article) return;
		try {
			await openExternalUrl(article.link);
		} catch (error) {
			uiStore.showToast(`Couldn't open original article: ${api.errorMessage(error)}`);
		}
	}

	async function shareArticle() {
		if (!article) return;
		try {
			const result = await shareArticleLink(article.title, article.link);
			if (result === 'copied') uiStore.showToast('Article link copied');
		} catch (error) {
			uiStore.showToast(`Couldn't share article: ${api.errorMessage(error)}`);
		}
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

	function moveToCategory() {
		if (!article) return;
		uiStore.openMoveCategory({ id: article.id, title: article.title });
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
	data-theme={theme}
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
					{theme}
					{hasOverride}
					onFontSize={setFontSize}
					onMeasure={setMeasure}
					onLeading={setLeading}
					onTheme={setTheme}
					onReset={resetOverrides}
				/>
				<button
					class="btn btn-icon btn-secondary share-btn"
					onclick={() => void shareArticle()}
					aria-label="Share article"
					title="Share article"
				>
					<Share />
				</button>
				<button
					class="btn btn-icon btn-secondary favorite-btn"
					class:favorited={article.favorited}
					onclick={toggleFavorite}
					aria-label="Favorite"
					title="Favorite"
				>
					<Star filled={article.favorited} />
				</button>
				<ArticleOverflowMenu
					{recapturing}
					onOpenOriginal={() => void openOriginal()}
					onRecapture={recapture}
					onMoveCategory={moveToCategory}
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
				<span>{article.category_name ?? 'Uncategorized'}</span>
				<span>·</span>
				<span>{formatReadTime(article.read_time_min)}</span>
				<span>·</span>
				<span>{formatCompactRelativeTime(article.published_at)}</span>
				<span>·</span>
				<a
					class="original-link"
					href={article.link}
					target="_blank"
					rel="noopener noreferrer"
					onclick={(event) => {
						event.preventDefault();
						void openOriginal();
					}}
				>
					<ExternalLink size={12} />
					View original
				</a>
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
		/* Own background/text color (not just inherited from `.content`)
		 *  so a per-article theme override (`data-theme` set above) actually
		 *  paints differently from the ambient app chrome around it — the
		 *  `[data-theme]` variable scoping lives in `tokens.css`. */
		background: var(--color-bg);
		color: var(--color-text);
	}
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
		flex-wrap: wrap;
		row-gap: 10px;
		margin: 0 0 20px;
		padding: calc(36px + env(safe-area-inset-top)) calc(36px + env(safe-area-inset-right)) 18px
			calc(36px + env(safe-area-inset-left));
		border-bottom: 1px solid var(--color-divider);
	}
	.header-row :global(.btn-secondary) {
		border-color: var(--color-divider);
		color: var(--color-text);
	}
	.header-row :global(.btn-secondary:hover:not(:disabled)) {
		background: color-mix(in srgb, var(--color-text) 7%, transparent);
	}
	.header-row :global(.seg) {
		border-color: var(--color-divider);
	}
	.header-row :global(.seg-opt) {
		color: var(--color-text);
		border-color: var(--color-divider);
	}
	.header-row :global(.seg-opt:not(:has(input:checked)):hover) {
		background: color-mix(in srgb, var(--color-text) 7%, transparent);
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
	.share-btn,
	.favorite-btn {
		color: var(--color-text);
	}
	.share-btn:hover:not(:disabled),
	.favorite-btn:hover:not(:disabled) {
		color: var(--color-accent);
	}
	.favorite-btn {
		color: var(--color-text);
	}
	.favorite-btn.favorited {
		color: var(--color-accent);
	}
	.hero {
		width: 100%;
		aspect-ratio: 21 / 9;
		max-height: 340px;
		margin-bottom: 28px;
		border-radius: var(--radius-lg);
		overflow: hidden;
		background: color-mix(in srgb, var(--color-text) 6%, transparent);
		border: 1px solid var(--color-divider);
	}
	.reader-meta {
		font-size: 13px;
		margin-bottom: 10px;
		color: var(--color-muted);
		flex-wrap: wrap;
	}
	.original-link {
		display: inline-flex;
		align-items: center;
		gap: 4px;
		font-weight: 600;
		white-space: nowrap;
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
