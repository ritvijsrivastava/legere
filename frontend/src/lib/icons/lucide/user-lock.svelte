<script lang="ts">
	let { size = 14 }: { size?: number } = $props();
</script>

<svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">
	<path d="M19 16v-2a2 2 0 0 0-4 0v2"></path>
	<path d="M9.5 15H7a4 4 0 0 0-4 4v2"></path>
	<circle cx="10" cy="7" r="4"></circle>
	<rect x="13" y="16" width="8" height="5" rx=".899"></rect>
</svg>
