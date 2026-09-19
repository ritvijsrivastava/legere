<script lang="ts">
	let { size = 14 }: { size?: number } = $props();
</script>

<svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">
	<path d="m8 11 2 2 4-4"></path>
	<circle cx="11" cy="11" r="8"></circle>
	<path d="m21 21-4.3-4.3"></path>
</svg>
