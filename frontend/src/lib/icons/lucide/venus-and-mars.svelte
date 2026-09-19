<script lang="ts">
	let { size = 14 }: { size?: number } = $props();
</script>

<svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">
	<path d="M10 20h4"></path>
	<path d="M12 16v6"></path>
	<path d="M17 2h4v4"></path>
	<path d="m21 2-5.46 5.46"></path>
	<circle cx="12" cy="11" r="5"></circle>
</svg>
