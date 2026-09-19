<script lang="ts">
	let { size = 14 }: { size?: number } = $props();
</script>

<svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">
	<path d="m15 15 6 6"></path>
	<path d="m15 9 6-6"></path>
	<path d="M21 16v5h-5"></path>
	<path d="M21 8V3h-5"></path>
	<path d="M3 16v5h5"></path>
	<path d="m3 21 6-6"></path>
	<path d="M3 8V3h5"></path>
	<path d="M9 9 3 3"></path>
</svg>
