<script lang="ts">
	let { size = 14 }: { size?: number } = $props();
</script>

<svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">
	<path d="M12 6v16"></path>
	<path d="m19 13 2-1a9 9 0 0 1-18 0l2 1"></path>
	<path d="M9 11h6"></path>
	<circle cx="12" cy="4" r="2"></circle>
</svg>
