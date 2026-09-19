<script lang="ts">
	let { size = 14 }: { size?: number } = $props();
</script>

<svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">
	<path d="M8 3H5a2 2 0 0 0-2 2v3"></path>
	<path d="M21 8V5a2 2 0 0 0-2-2h-3"></path>
	<path d="M3 16v3a2 2 0 0 0 2 2h3"></path>
	<path d="M16 21h3a2 2 0 0 0 2-2v-3"></path>
</svg>
