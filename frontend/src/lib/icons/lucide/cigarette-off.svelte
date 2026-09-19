<script lang="ts">
	let { size = 14 }: { size?: number } = $props();
</script>

<svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">
	<path d="M12 12H3a1 1 0 0 0-1 1v2a1 1 0 0 0 1 1h13"></path>
	<path d="M18 8c0-2.5-2-2.5-2-5"></path>
	<path d="m2 2 20 20"></path>
	<path d="M21 12a1 1 0 0 1 1 1v2a1 1 0 0 1-.5.866"></path>
	<path d="M22 8c0-2.5-2-2.5-2-5"></path>
	<path d="M7 12v4"></path>
</svg>
