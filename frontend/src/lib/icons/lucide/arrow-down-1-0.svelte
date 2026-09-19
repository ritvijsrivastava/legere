<script lang="ts">
	let { size = 14 }: { size?: number } = $props();
</script>

<svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">
	<path d="m3 16 4 4 4-4"></path>
	<path d="M7 20V4"></path>
	<path d="M17 10V4h-2"></path>
	<path d="M15 10h4"></path>
	<rect x="15" y="14" width="4" height="6" ry="2"></rect>
</svg>
