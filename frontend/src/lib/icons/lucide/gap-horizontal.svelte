<script lang="ts">
	let { size = 14 }: { size?: number } = $props();
</script>

<svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">
	<path d="M12 2v2"></path>
	<path d="M12 8v2"></path>
	<path d="M12 14v2"></path>
	<path d="M12 20v2"></path>
	<path d="M21 3h-3a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h3"></path>
	<path d="M3 3h3a2 2 0 0 1 2 2v14a2 2 0 0 1-2 2H3"></path>
</svg>
