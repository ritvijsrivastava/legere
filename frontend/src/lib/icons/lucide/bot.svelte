<script lang="ts">
	let { size = 14 }: { size?: number } = $props();
</script>

<svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">
	<path d="M12 8V4H8"></path>
	<rect width="16" height="12" x="4" y="8" rx="2"></rect>
	<path d="M2 14h2"></path>
	<path d="M20 14h2"></path>
	<path d="M15 13v2"></path>
	<path d="M9 13v2"></path>
</svg>
