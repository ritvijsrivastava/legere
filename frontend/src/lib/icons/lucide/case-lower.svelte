<script lang="ts">
	let { size = 14 }: { size?: number } = $props();
</script>

<svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">
	<path d="M10 9v7"></path>
	<path d="M14 6v10"></path>
	<circle cx="17.5" cy="12.5" r="3.5"></circle>
	<circle cx="6.5" cy="12.5" r="3.5"></circle>
</svg>
