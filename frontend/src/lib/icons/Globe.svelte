<script lang="ts">
	let { size = 14 }: { size?: number } = $props();
</script>

<svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">
	<circle cx="12" cy="12" r="9"></circle>
	<line x1="3" y1="12" x2="21" y2="12"></line>
	<path d="M12 3a15 15 0 0 1 0 18a15 15 0 0 1 0-18Z"></path>
</svg>
