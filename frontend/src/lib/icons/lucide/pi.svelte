<script lang="ts">
	let { size = 14 }: { size?: number } = $props();
</script>

<svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">
	<line x1="9" x2="9" y1="4" y2="20"></line>
	<path d="M4 7c0-1.7 1.3-3 3-3h13"></path>
	<path d="M18 20c-1.7 0-3-1.3-3-3V4"></path>
</svg>
