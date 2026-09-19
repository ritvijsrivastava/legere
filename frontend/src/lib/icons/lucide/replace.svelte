<script lang="ts">
	let { size = 14 }: { size?: number } = $props();
</script>

<svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">
	<path d="M14 4a1 1 0 0 1 1-1"></path>
	<path d="M15 10a1 1 0 0 1-1-1"></path>
	<path d="M21 4a1 1 0 0 0-1-1"></path>
	<path d="M21 9a1 1 0 0 1-1 1"></path>
	<path d="m3 7 3 3 3-3"></path>
	<path d="M6 10V5a2 2 0 0 1 2-2h2"></path>
	<rect x="3" y="14" width="7" height="7" rx="1"></rect>
</svg>
