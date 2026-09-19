<script lang="ts">
	let { size = 14 }: { size?: number } = $props();
</script>

<svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">
	<path d="M11 14h10"></path>
	<path d="M16 4h2a2 2 0 0 1 2 2v1.344"></path>
	<path d="m17 18 4-4-4-4"></path>
	<path d="M8 4H6a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h12a2 2 0 0 0 1.793-1.113"></path>
	<rect x="8" y="2" width="8" height="4" rx="1"></rect>
</svg>
