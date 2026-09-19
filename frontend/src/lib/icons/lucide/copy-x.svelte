<script lang="ts">
	let { size = 14 }: { size?: number } = $props();
</script>

<svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">
	<path d="M4 16a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h10a2 2 0 0 1 2 2"></path>
	<rect x="8" y="8" width="14" height="14" rx="2"></rect>
	<path d="m12.5 12.5 5 5"></path>
	<path d="m12.5 17.5 5-5"></path>
</svg>
