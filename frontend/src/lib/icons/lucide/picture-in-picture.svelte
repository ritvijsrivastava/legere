<script lang="ts">
	let { size = 14 }: { size?: number } = $props();
</script>

<svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">
	<path d="M2 10h6V4"></path>
	<path d="m2 4 6 6"></path>
	<path d="M21 10V7a2 2 0 0 0-2-2h-7"></path>
	<path d="M3 14v2a2 2 0 0 0 2 2h3"></path>
	<rect x="12" y="14" width="10" height="7" rx="1"></rect>
</svg>
