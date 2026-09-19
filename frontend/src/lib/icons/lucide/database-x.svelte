<script lang="ts">
	let { size = 14 }: { size?: number } = $props();
</script>

<svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">
	<path d="m17 17 5 5"></path>
	<path d="M19.323 13.744A9 3 0 0 0 21 12"></path>
	<path d="M21 13.127V5"></path>
	<path d="m22 17-5 5"></path>
	<path d="M3 12A9 3 0 0 0 13.563 14.954"></path>
	<path d="M3 5V19A9 3 0 0 0 13 21.981"></path>
	<ellipse cx="12" cy="5" rx="9" ry="3"></ellipse>
</svg>
