<script lang="ts">
	let { size = 14 }: { size?: number } = $props();
</script>

<svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">
	<rect width="16" height="20" x="4" y="2" rx="2"></rect>
	<path d="M12 6h.01"></path>
	<circle cx="12" cy="14" r="4"></circle>
	<path d="M12 14h.01"></path>
</svg>
