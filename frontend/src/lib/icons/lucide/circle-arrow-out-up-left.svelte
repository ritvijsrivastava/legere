<script lang="ts">
	let { size = 14 }: { size?: number } = $props();
</script>

<svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">
	<path d="M2 8V2h6"></path>
	<path d="m2 2 10 10"></path>
	<path d="M12 2A10 10 0 1 1 2 12"></path>
</svg>
