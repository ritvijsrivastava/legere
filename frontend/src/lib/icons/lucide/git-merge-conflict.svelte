<script lang="ts">
	let { size = 14 }: { size?: number } = $props();
</script>

<svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">
	<path d="M12 6h4a2 2 0 0 1 2 2v7"></path>
	<path d="M6 12v9"></path>
	<path d="m8.5 3.5-5 5"></path>
	<path d="m8.5 8.5-5-5"></path>
	<circle cx="18" cy="18" r="3"></circle>
</svg>
