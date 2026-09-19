<script lang="ts">
	let { size = 14 }: { size?: number } = $props();
</script>

<svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">
	<path d="m16 19 3 3 3-3"></path>
	<path d="M19 16v6"></path>
	<path d="M21 12.536V5"></path>
	<path d="M3 12A9 3 0 0 0 15.182 14.806"></path>
	<path d="M3 5V19A9 3 0 0 0 13.318 21.968"></path>
	<ellipse cx="12" cy="5" rx="9" ry="3"></ellipse>
</svg>
