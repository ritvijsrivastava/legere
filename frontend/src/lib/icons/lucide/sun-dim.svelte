<script lang="ts">
	let { size = 14 }: { size?: number } = $props();
</script>

<svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">
	<circle cx="12" cy="12" r="4"></circle>
	<path d="M12 4h.01"></path>
	<path d="M20 12h.01"></path>
	<path d="M12 20h.01"></path>
	<path d="M4 12h.01"></path>
	<path d="M17.657 6.343h.01"></path>
	<path d="M17.657 17.657h.01"></path>
	<path d="M6.343 17.657h.01"></path>
	<path d="M6.343 6.343h.01"></path>
</svg>
