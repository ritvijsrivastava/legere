<script lang="ts">
	let { size = 14 }: { size?: number } = $props();
</script>

<svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">
	<path d="M8 2v3"></path>
	<path d="M16 2v3"></path>
	<rect x="3" y="3" width="18" height="18" rx="2"></rect>
	<path d="M3 9h18"></path>
	<path d="m9 15 2 2 4-4"></path>
</svg>
