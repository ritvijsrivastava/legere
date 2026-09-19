<script lang="ts">
	let { size = 14 }: { size?: number } = $props();
</script>

<svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">
	<rect width="14" height="20" x="5" y="2" rx="2"></rect>
	<path d="M15 14h.01"></path>
	<path d="M9 6h6"></path>
	<path d="M9 10h6"></path>
</svg>
