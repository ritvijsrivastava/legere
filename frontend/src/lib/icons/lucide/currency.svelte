<script lang="ts">
	let { size = 14 }: { size?: number } = $props();
</script>

<svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">
	<circle cx="12" cy="12" r="8"></circle>
	<line x1="3" x2="6" y1="3" y2="6"></line>
	<line x1="21" x2="18" y1="3" y2="6"></line>
	<line x1="3" x2="6" y1="21" y2="18"></line>
	<line x1="21" x2="18" y1="21" y2="18"></line>
</svg>
