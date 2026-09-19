<script lang="ts">
	let { size = 14 }: { size?: number } = $props();
</script>

<svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">
	<path d="M12 3v6"></path>
	<circle cx="12" cy="12" r="3"></circle>
	<path d="M12 15v6"></path>
</svg>
