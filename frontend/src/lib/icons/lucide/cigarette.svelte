<script lang="ts">
	let { size = 14 }: { size?: number } = $props();
</script>

<svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">
	<path d="M17 12H3a1 1 0 0 0-1 1v2a1 1 0 0 0 1 1h14"></path>
	<path d="M18 8c0-2.5-2-2.5-2-5"></path>
	<path d="M21 16a1 1 0 0 0 1-1v-2a1 1 0 0 0-1-1"></path>
	<path d="M22 8c0-2.5-2-2.5-2-5"></path>
	<path d="M7 12v4"></path>
</svg>
