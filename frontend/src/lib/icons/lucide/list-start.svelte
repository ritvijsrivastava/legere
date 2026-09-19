<script lang="ts">
	let { size = 14 }: { size?: number } = $props();
</script>

<svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">
	<path d="M3 5h6"></path>
	<path d="M3 12h13"></path>
	<path d="M3 19h13"></path>
	<path d="m16 8-3-3 3-3"></path>
	<path d="M21 19V7a2 2 0 0 0-2-2h-6"></path>
</svg>
