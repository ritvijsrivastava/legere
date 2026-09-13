<script lang="ts">
	import { assetUrl } from '$lib/api';

	let { path, alt = '' }: { path: string | null; alt?: string } = $props();

	let rootEl = $state<HTMLElement | null>(null);
	// Gated by IntersectionObserver rather than fetched on mount — with
	// hundreds/thousands of cards rendered at once (no virtualization yet),
	// firing the assetUrl IPC round-trip (and the resulting disk read) for
	// every offscreen card at once was a major source of jank. `rootMargin`
	// gives a preload buffer so images are ready just before they scroll
	// into view.
	let visible = $state(false);
	let src = $state<string | null>(null);

	$effect(() => {
		if (!rootEl || typeof IntersectionObserver === 'undefined') {
			visible = true;
			return;
		}
		const observer = new IntersectionObserver(
			(entries) => {
				if (entries.some((entry) => entry.isIntersecting)) {
					visible = true;
					observer.disconnect();
				}
			},
			{ rootMargin: '200px' }
		);
		observer.observe(rootEl);
		return () => observer.disconnect();
	});

	$effect(() => {
		if (!visible) return;
		let cancelled = false;
		assetUrl(path).then((url) => {
			if (!cancelled) src = url;
		});
		return () => {
			cancelled = true;
		};
	});
</script>

<div class="lighten hero-image" bind:this={rootEl}>
	{#if src}
		<img {src} {alt} loading="lazy" decoding="async" />
	{/if}
</div>

<style>
	.hero-image {
		width: 100%;
		height: 100%;
		background: var(--color-surface);
	}
	.hero-image img {
		width: 100%;
		height: 100%;
		object-fit: cover;
		animation: hero-in var(--duration-base) var(--ease-snap);
	}
	@keyframes hero-in {
		from { opacity: 0; }
	}
</style>
