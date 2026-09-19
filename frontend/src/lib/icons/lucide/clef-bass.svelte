<script lang="ts">
	let { size = 14 }: { size?: number } = $props();
</script>

<svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">
	<path d="M19 11h.01"></path>
	<path d="M19 6h.01"></path>
	<path d="M5 8c0-4 4-4 4-4 6 0 6 6 6 6 0 7-10 11-10 11"></path>
	<circle cx="7" cy="8" r="2"></circle>
</svg>
