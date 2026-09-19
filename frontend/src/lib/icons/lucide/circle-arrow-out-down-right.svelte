<script lang="ts">
	let { size = 14 }: { size?: number } = $props();
</script>

<svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">
	<path d="M12 22a10 10 0 1 1 10-10"></path>
	<path d="M22 22 12 12"></path>
	<path d="M22 16v6h-6"></path>
</svg>
