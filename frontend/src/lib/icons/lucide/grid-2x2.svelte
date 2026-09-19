<script lang="ts">
	let { size = 14 }: { size?: number } = $props();
</script>

<svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">
	<path d="M12 3v18"></path>
	<path d="M3 12h18"></path>
	<rect x="3" y="3" width="18" height="18" rx="2"></rect>
</svg>
