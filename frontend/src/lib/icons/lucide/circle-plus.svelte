<script lang="ts">
	let { size = 14 }: { size?: number } = $props();
</script>

<svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">
	<circle cx="12" cy="12" r="10"></circle>
	<path d="M8 12h8"></path>
	<path d="M12 8v8"></path>
</svg>
