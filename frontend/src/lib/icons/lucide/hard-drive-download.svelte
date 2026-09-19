<script lang="ts">
	let { size = 14 }: { size?: number } = $props();
</script>

<svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">
	<path d="M12 2v8"></path>
	<path d="m16 6-4 4-4-4"></path>
	<rect width="20" height="8" x="2" y="14" rx="2"></rect>
	<path d="M6 18h.01"></path>
	<path d="M10 18h.01"></path>
</svg>
