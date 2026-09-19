<script lang="ts">
	let { size = 14 }: { size?: number } = $props();
</script>

<svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">
	<path d="M12 15v7"></path>
	<path d="M9 19h6"></path>
	<circle cx="12" cy="9" r="6"></circle>
</svg>
