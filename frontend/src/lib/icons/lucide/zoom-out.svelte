<script lang="ts">
	let { size = 14 }: { size?: number } = $props();
</script>

<svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">
	<circle cx="11" cy="11" r="8"></circle>
	<line x1="21" x2="16.65" y1="21" y2="16.65"></line>
	<line x1="8" x2="14" y1="11" y2="11"></line>
</svg>
