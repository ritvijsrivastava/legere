<script lang="ts">
	let { size = 14 }: { size?: number } = $props();
</script>

<svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">
	<circle cx="5" cy="6" r="3"></circle>
	<path d="M5 9v12"></path>
	<circle cx="19" cy="18" r="3"></circle>
	<path d="m15 9-3-3 3-3"></path>
	<path d="M12 6h5a2 2 0 0 1 2 2v7"></path>
</svg>
