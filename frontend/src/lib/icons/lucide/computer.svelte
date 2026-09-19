<script lang="ts">
	let { size = 14 }: { size?: number } = $props();
</script>

<svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">
	<path d="M12 18h6"></path>
	<path d="M6 18h.01"></path>
	<path d="M8 6h1"></path>
	<rect x="2" y="14" width="20" height="8" rx="2"></rect>
	<rect x="4" y="2" width="16" height="12" rx="2"></rect>
</svg>
