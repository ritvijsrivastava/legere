<script lang="ts">
	let { size = 15 }: { size?: number } = $props();
</script>

<svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">
	<rect x="3" y="3" width="8" height="8"></rect>
	<rect x="13" y="3" width="8" height="8"></rect>
	<rect x="3" y="13" width="8" height="8"></rect>
	<rect x="13" y="13" width="8" height="8"></rect>
</svg>
