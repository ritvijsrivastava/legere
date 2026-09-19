<script lang="ts">
	let { size = 14 }: { size?: number } = $props();
</script>

<svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">
	<rect width="18" height="12" x="3" y="8" rx="1"></rect>
	<path d="M10 8V5c0-.6-.4-1-1-1H6a1 1 0 0 0-1 1v3"></path>
	<path d="M19 8V5c0-.6-.4-1-1-1h-3a1 1 0 0 0-1 1v3"></path>
</svg>
