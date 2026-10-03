<script lang="ts">
	import { page } from '$app/state';
	import { goto } from '$app/navigation';
	import { libraryStatsStore } from '$lib/stores/libraryStats.svelte';
	import { settingsStore } from '$lib/stores/settings.svelte';
	import * as api from '$lib/api';
	import type {
		AppTheme,
		ArticleDetail,
		ReaderMeasure,
		ReaderLeading,
		ReaderFont,
		ReadingOverrides
	} from '$lib/types';
	import HeroImage from '$lib/components/HeroImage.svelte';
	import ReaderControls from '$lib/components/ReaderControls.svelte';
	import TagEditor from '$lib/components/TagEditor.svelte';
	import ArticleOverflowMenu from '$lib/components/ArticleOverflowMenu.svelte';
	import { uiStore } from '$lib/stores/ui.svelte';
	import { openExternalUrl, shareArticleLink } from '$lib/articleActions';
	import ChevronLeft from '$lib/icons/ChevronLeft.svelte';
	import ExternalLink from '$lib/icons/ExternalLink.svelte';
	import FolderMove from '$lib/icons/FolderMove.svelte';
	import Refresh from '$lib/icons/Refresh.svelte';
	import Share from '$lib/icons/Share.svelte';
	import Star from '$lib/icons/Star.svelte';
	import TriangleAlert from '$lib/icons/TriangleAlert.svelte';
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

	let fontSize = $state(16);
	let measure = $state<ReaderMeasure>('default');
	let leading = $state<ReaderLeading>('default');
	let theme = $state<AppTheme>('dark');
	let font = $state<ReaderFont>('libron');
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
		font = article.overrides.font ?? settingsStore.current.reader_font;
	});

	let hasOverride = $derived(
		article !== null &&
			(article.overrides.font_size !== null ||
				article.overrides.measure !== null ||
				article.overrides.leading !== null ||
				article.overrides.theme !== null ||
				article.overrides.font !== null)
	);

	$effect(() => {
		const id = page.params.id;
		if (!id) return;
		article = null;
		restoredScrollForId = null;
		api.openForReading(id).then((detail) => {
			// Navigated to a different article while this one was loading.
			if (page.params.id !== id) return;
			article = detail;
			// A failed capture gets a fresh attempt on *every* open, not just
			// the first: the failure is often transient (network down at
			// share/sync time), and a successful retry swaps the real
			// article in right here. If it fails again, the banner just
			// shows the newest error. An article with no content at all is
			// treated the same even when it isn't flagged failed — rows
			// synced from a device that predates the synced `capture_failed`
			// flag arrive as exactly that: empty, unflagged.
			if (detail.capture_failed || !detail.content_html.trim()) {
				void recapture({ automatic: true });
			}
		});
	});

	/** Nothing readable stored — either a flagged failed capture or an
	 *  unflagged empty one (see the retry in the effect above). */
	let contentMissing = $derived(article !== null && !article.content_html.trim());

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
	function setFont(value: ReaderFont) {
		font = value;
		persistOverrides({ font: value });
	}
	/** Clears every override on the current article, falling back to
	 *  whatever the global settings are right now. */
	function resetOverrides() {
		if (!article) return;
		const overrides: ReadingOverrides = {
			font_size: null,
			measure: null,
			leading: null,
			theme: null,
			font: null
		};
		article = { ...article, overrides };
		api.setReadingOverrides(article.id, overrides);
		fontSize = settingsStore.current.reader_font_size;
		measure = settingsStore.current.reader_measure;
		leading = settingsStore.current.reader_leading;
		theme = settingsStore.current.app_theme;
		font = settingsStore.current.reader_font;
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

	// Which article a capture is in flight for — an id rather than a plain
	// boolean so leaving mid-retry doesn't leave the *next* article's reader
	// looking busy (or, worse, blocked from retrying itself).
	let recapturingId = $state<string | null>(null);
	let recapturing = $derived(article !== null && recapturingId === article.id);

	async function recapture(options?: { automatic?: boolean }) {
		if (!article || recapturingId === article.id) return;
		const id = article.id;
		recapturingId = id;
		try {
			const detail = await api.recaptureArticle(id);
			if (article?.id === id) article = detail;
		} catch (error) {
			// An automatic retry that errors out (rather than just failing
			// to capture, which comes back as a normal result) leaves the
			// existing banner in place; only an explicit "Re-capture"
			// surfaces the error.
			if (!options?.automatic) {
				uiStore.showToast(`Couldn't re-capture: ${api.errorMessage(error)}`);
			}
		} finally {
			if (recapturingId === id) recapturingId = null;
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
	data-reading-font={font}
>
	<div class="progress-track">
		<div class="progress-fill" style:width="{Math.round(scrollProgress * 100)}%"></div>
	</div>

	<div class="header-row">
		<button class="btn btn-ghost back-btn" onclick={() => goto(backHref)}>
			<ChevronLeft />
			<span class="back-label">{backLabel}</span>
		</button>
		{#if article}
			<div class="controls">
				<ReaderControls
					{fontSize}
					{measure}
					{leading}
					{theme}
					{font}
					{hasOverride}
					onFontSize={setFontSize}
					onMeasure={setMeasure}
					onLeading={setLeading}
					onTheme={setTheme}
					onFont={setFont}
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
				<button
					class="btn btn-icon btn-secondary move-btn"
					onclick={moveToCategory}
					aria-label="Move to category"
					title="Move to category"
				>
					<FolderMove size={16} />
				</button>
				<ArticleOverflowMenu {recapturing} onRecapture={() => void recapture()} onDelete={deleteArticle} />
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

			{#if (article.capture_failed || contentMissing) && recapturing}
				<div class="capture-retrying text-muted" role="status">
					<Refresh size={14} spinning />
					<span>Couldn't save this article earlier — trying again…</span>
				</div>
			{:else if article.capture_failed || contentMissing}
				<div class="capture-error" role="alert">
					<TriangleAlert size={16} />
					<div class="capture-error-body">
						<p class="capture-error-title">Couldn't save a readable copy of this article</p>
						<p class="capture-error-message">{article.capture_error ??
							(article.capture_failed ? 'Unknown error' : 'The page had no readable content.')}</p>
						<a
							class="capture-error-link"
							href={article.link}
							target="_blank"
							rel="noopener noreferrer"
							onclick={(event) => {
								event.preventDefault();
								void openOriginal();
							}}
						>
							<ExternalLink size={12} />
							Open the original link instead
						</a>
					</div>
				</div>
			{:else}
				<div
					class="reader-body"
					style:font-size="{fontSize}px"
					style:line-height={LEADING_VALUE[leading]}
				>
					{@html resolvedContentHtml}
				</div>
			{/if}
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
	.favorite-btn,
	.move-btn {
		color: var(--color-text);
	}
	.share-btn:hover:not(:disabled),
	.favorite-btn:hover:not(:disabled),
	.move-btn:hover:not(:disabled) {
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
		font-weight: 600;
		font-size: clamp(28px, 5vw, 38px);
		line-height: 1.15;
		letter-spacing: -0.01em;
		margin: 0 0 28px;
		overflow-wrap: anywhere;
	}
	.reader-body {
		font-size: 16px;
	}
	.capture-retrying {
		display: flex;
		align-items: center;
		gap: 10px;
		padding: 16px;
		font-size: 14px;
	}
	.capture-error {
		display: flex;
		align-items: flex-start;
		gap: 12px;
		padding: 16px;
		border-radius: var(--radius-lg);
		background: color-mix(in srgb, var(--color-danger) 10%, var(--color-surface-raised));
		box-shadow: 0 0 0 1px color-mix(in srgb, var(--color-danger) 35%, transparent);
		color: var(--color-danger);
	}
	.capture-error :global(svg) {
		flex-shrink: 0;
		margin-top: 2px;
	}
	.capture-error-body {
		display: flex;
		flex-direction: column;
		gap: 6px;
		min-width: 0;
	}
	.capture-error-title {
		margin: 0;
		font-weight: 600;
		font-size: 15px;
		color: var(--color-text);
	}
	.capture-error-message {
		margin: 0;
		font-size: 13px;
		word-break: break-word;
	}
	.capture-error-link {
		display: inline-flex;
		align-items: center;
		gap: 4px;
		margin-top: 4px;
		font-size: 13px;
		font-weight: 600;
		width: fit-content;
	}
	.capture-error-link:hover {
		text-decoration: underline;
	}

	/* The rest of the app follows a 20px 16px / 32px mobile padding rhythm
	   (see DESIGN.md) — the reader never picked it up and sat at the full
	   desktop 36px/56px margins even on a ~360–430px phone, which is the
	   one surface where every pixel of measure matters most. */
	.back-btn :global(svg) {
		flex: none;
	}
	.back-label {
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	@media (max-width: 768px) {
		.header-row {
			padding: calc(16px + env(safe-area-inset-top)) calc(16px + env(safe-area-inset-right)) 14px
				calc(16px + env(safe-area-inset-left));
			margin: 0 0 16px;
			/* One line, always: back button on the left, the five action
			   buttons on the right. Wrapping here is what dropped the icon
			   row under "Library" on phones ~360-370px wide, where the
			   desktop's 10px control gap overshot the row by a few pixels.
			   The 6px gap below brings the controls to ~244px; the back
			   button gives up width (label ellipsizes) before anything
			   wraps, so a long category name can't push it either. */
			flex-wrap: nowrap;
			gap: 10px;
		}
		.back-btn {
			min-width: 0;
		}
		.controls {
			flex: none;
			flex-wrap: nowrap;
			gap: 6px;
		}
		.reader-page {
			padding: 0 calc(16px + env(safe-area-inset-right)) 40px calc(16px + env(safe-area-inset-left));
		}
		.hero {
			margin-bottom: 20px;
		}
		.reader-tags {
			margin: 0 0 20px;
		}
		.reader-title {
			margin: 0 0 18px;
		}
	}
</style>
