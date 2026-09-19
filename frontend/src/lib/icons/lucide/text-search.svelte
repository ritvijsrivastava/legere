<script lang="ts">
	let { size = 14 }: { size?: number } = $props();
</script>

<svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">
	<path d="M21 5H3"></path>
	<path d="M10 12H3"></path>
	<path d="M10 19H3"></path>
	<circle cx="17" cy="15" r="3"></circle>
	<path d="m21 19-1.9-1.9"></path>
</svg>
