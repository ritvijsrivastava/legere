<script lang="ts">
	let { size = 14 }: { size?: number } = $props();
</script>

<svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">
	<path d="M12 3v18"></path>
	<path d="m16 16 4-4-4-4"></path>
	<path d="m8 8-4 4 4 4"></path>
</svg>
