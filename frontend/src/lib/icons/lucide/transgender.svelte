<script lang="ts">
	let { size = 14 }: { size?: number } = $props();
</script>

<svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">
	<path d="M12 16v6"></path>
	<path d="M14 20h-4"></path>
	<path d="M18 2h4v4"></path>
	<path d="m2 2 7.17 7.17"></path>
	<path d="M2 5.355V2h3.357"></path>
	<path d="m22 2-7.17 7.17"></path>
	<path d="M8 5 5 8"></path>
	<circle cx="12" cy="12" r="4"></circle>
</svg>
