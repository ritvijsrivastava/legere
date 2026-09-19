<script lang="ts">
	let { size = 14 }: { size?: number } = $props();
</script>

<svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">
	<path d="M10 12.01h.01"></path>
	<path d="M18 8v4a8 8 0 0 1-1.07 4"></path>
	<circle cx="10" cy="12" r="4"></circle>
	<rect x="2" y="4" width="20" height="16" rx="2"></rect>
</svg>
