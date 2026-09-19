<script lang="ts">
	let { size = 14 }: { size?: number } = $props();
</script>

<svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">
	<path d="M16 5H3"></path>
	<path d="M11 12H3"></path>
	<path d="M11 19H3"></path>
	<path d="M21 16V5"></path>
	<circle cx="18" cy="16" r="3"></circle>
</svg>
