<script lang="ts">
	let { size = 14 }: { size?: number } = $props();
</script>

<svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">
	<path d="m10 9-3 3 3 3"></path>
	<path d="m14 15 3-3-3-3"></path>
	<rect x="3" y="3" width="18" height="18" rx="2"></rect>
</svg>
