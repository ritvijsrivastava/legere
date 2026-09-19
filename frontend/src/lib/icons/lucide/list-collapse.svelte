<script lang="ts">
	let { size = 14 }: { size?: number } = $props();
</script>

<svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">
	<path d="M10 5h11"></path>
	<path d="M10 12h11"></path>
	<path d="M10 19h11"></path>
	<path d="m3 10 3-3-3-3"></path>
	<path d="m3 20 3-3-3-3"></path>
</svg>
