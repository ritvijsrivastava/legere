<script lang="ts">
	let { size = 14 }: { size?: number } = $props();
</script>

<svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">
	<path d="M2 12 7 2"></path>
	<path d="m7 12 5-10"></path>
	<path d="m12 12 5-10"></path>
	<path d="m17 12 5-10"></path>
	<path d="M4.5 7h15"></path>
	<path d="M12 16v6"></path>
</svg>
