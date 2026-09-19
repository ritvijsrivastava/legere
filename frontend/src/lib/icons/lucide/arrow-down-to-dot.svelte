<script lang="ts">
	let { size = 14 }: { size?: number } = $props();
</script>

<svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">
	<path d="M12 2v14"></path>
	<path d="m19 9-7 7-7-7"></path>
	<circle cx="12" cy="21" r="1"></circle>
</svg>
