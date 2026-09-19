<script lang="ts">
	let { size = 14 }: { size?: number } = $props();
</script>

<svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">
	<path d="M5 16v2"></path>
	<path d="M19 16v2"></path>
	<rect width="20" height="8" x="2" y="8" rx="2"></rect>
	<path d="M18 12h.01"></path>
</svg>
