<script lang="ts">
	let { size = 14 }: { size?: number } = $props();
</script>

<svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">
	<rect width="7" height="13" x="3" y="8" rx="1"></rect>
	<path d="m15 2-3 3-3-3"></path>
	<rect width="7" height="13" x="14" y="8" rx="1"></rect>
</svg>
