<script lang="ts">
	let { size = 14 }: { size?: number } = $props();
</script>

<svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">
	<circle cx="10" cy="7" r="4"></circle>
	<path d="M10.3 15H7a4 4 0 0 0-4 4v2"></path>
	<circle cx="17" cy="17" r="3"></circle>
	<path d="m21 21-1.9-1.9"></path>
</svg>
