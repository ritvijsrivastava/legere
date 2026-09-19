<script lang="ts">
	// Renders one icon from the full lazily-loaded Lucide set
	// (`lib/lucideIcons.ts`) by id — the counterpart to `CategoryIcon`'s
	// fast synchronous path for the curated pack, used when a category's
	// `icon` is one of the other ~1800 icons a user picked via
	// `CategoryIconPicker`'s search. Shows the generic folder glyph until
	// the chunk loads (near-instant, it's a local bundle) or if the id is
	// unrecognized, rather than rendering nothing.
	import type { Component } from 'svelte';
	import { loadLucideIcon } from '$lib/lucideIcons';
	import Folder from '../icons/Folder.svelte';

	let { id, size = 14 }: { id: string; size?: number } = $props();

	let Icon = $state<Component<{ size?: number }> | null>(null);

	$effect(() => {
		const wanted = id;
		Icon = null;
		loadLucideIcon(wanted)
			.then((C) => {
				if (id === wanted) Icon = C;
			})
			.catch(() => {
				if (id === wanted) Icon = null;
			});
	});
</script>

{#if Icon}
	{@const Loaded = Icon}
	<Loaded {size} />
{:else}
	<Folder {size} />
{/if}
