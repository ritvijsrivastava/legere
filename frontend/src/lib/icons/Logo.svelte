<script lang="ts">
	/** The Legere mark: a bookmark ribbon, standing in for "keep this to
	 *  read" — the one durable thing the product actually does. Filled in
	 *  `currentColor` so it inherits the accent wherever it's placed. */
	let { size = 18 }: { size?: number } = $props();
</script>

<svg width={size} height={size} viewBox="0 0 24 24" fill="none" aria-hidden="true">
	<path
		d="M8 3.5H16C16.2761 3.5 16.5 3.72386 16.5 4V19.5L12 16.85L7.5 19.5V4C7.5 3.72386 7.72386 3.5 8 3.5Z"
		fill="currentColor"
	/>
</svg>
