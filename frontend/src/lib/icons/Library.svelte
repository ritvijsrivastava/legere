<script lang="ts">
	let { size = 17 }: { size?: number } = $props();
</script>

<svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">
	<rect x="3" y="4" width="18" height="4"></rect>
	<rect x="3" y="10" width="18" height="4"></rect>
	<rect x="3" y="16" width="18" height="4"></rect>
</svg>
