<script lang="ts">
	let { size = 14 }: { size?: number } = $props();
</script>

<svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">
	<path d="M12 2v20"></path>
	<path d="M2 5h20"></path>
	<path d="M3 3v2"></path>
	<path d="M7 3v2"></path>
	<path d="M17 3v2"></path>
	<path d="M21 3v2"></path>
	<path d="m19 5-7 7-7-7"></path>
</svg>
