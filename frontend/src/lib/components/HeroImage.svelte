<script lang="ts">
	import { assetUrl } from '$lib/api';

	let { path, alt = '' }: { path: string | null; alt?: string } = $props();

	let src = $state<string | null>(null);

	$effect(() => {
		let cancelled = false;
		assetUrl(path).then((url) => {
			if (!cancelled) src = url;
		});
		return () => {
			cancelled = true;
		};
	});
</script>

<div class="lighten hero-image">
	{#if src}
		<img {src} {alt} />
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
	}
</style>
