<script lang="ts">
	let { size = 14 }: { size?: number } = $props();
</script>

<svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">
	<path d="M21 11.693V5"></path>
	<path d="m22 22-1.875-1.875"></path>
	<path d="M3 12a9 3 0 0 0 8.697 2.998"></path>
	<path d="M3 5v14a9 3 0 0 0 9.28 2.999"></path>
	<circle cx="18" cy="18" r="3"></circle>
	<ellipse cx="12" cy="5" rx="9" ry="3"></ellipse>
</svg>
