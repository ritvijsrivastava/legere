<script lang="ts">
	let { size = 14 }: { size?: number } = $props();
</script>

<svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">
	<path d="M21 14.5A9 6.5 0 0 1 5.5 19"></path>
	<path d="M3 9.5A9 6.5 0 0 1 18.5 5"></path>
	<circle cx="17.5" cy="14.5" r="3.5"></circle>
	<circle cx="6.5" cy="9.5" r="3.5"></circle>
</svg>
