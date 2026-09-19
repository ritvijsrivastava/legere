<script lang="ts">
	let { size = 14 }: { size?: number } = $props();
</script>

<svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">
	<path d="M12 2v2"></path>
	<path d="M12 8a4 4 0 0 0-1.645 7.647"></path>
	<path d="M2 12h2"></path>
	<path d="M20 14.54a4 4 0 1 1-4 0V4a2 2 0 0 1 4 0z"></path>
	<path d="m4.93 4.93 1.41 1.41"></path>
	<path d="m6.34 17.66-1.41 1.41"></path>
</svg>
