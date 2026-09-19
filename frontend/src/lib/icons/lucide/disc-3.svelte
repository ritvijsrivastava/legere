<script lang="ts">
	let { size = 14 }: { size?: number } = $props();
</script>

<svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">
	<circle cx="12" cy="12" r="10"></circle>
	<path d="M6 12c0-1.7.7-3.2 1.8-4.2"></path>
	<circle cx="12" cy="12" r="2"></circle>
	<path d="M18 12c0 1.7-.7 3.2-1.8 4.2"></path>
</svg>
