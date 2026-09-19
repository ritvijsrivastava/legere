<script lang="ts">
	let { size = 14 }: { size?: number } = $props();
</script>

<svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">
	<rect x="2" y="4" width="20" height="16" rx="2"></rect>
	<path d="M10 4v4"></path>
	<path d="M2 8h20"></path>
	<path d="M6 4v4"></path>
</svg>
