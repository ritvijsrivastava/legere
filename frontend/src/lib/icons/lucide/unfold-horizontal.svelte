<script lang="ts">
	let { size = 14 }: { size?: number } = $props();
</script>

<svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">
	<path d="M16 12h6"></path>
	<path d="M8 12H2"></path>
	<path d="M12 2v2"></path>
	<path d="M12 8v2"></path>
	<path d="M12 14v2"></path>
	<path d="M12 20v2"></path>
	<path d="m19 15 3-3-3-3"></path>
	<path d="m5 9-3 3 3 3"></path>
</svg>
