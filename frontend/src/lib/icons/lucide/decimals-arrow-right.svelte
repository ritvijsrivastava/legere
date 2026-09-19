<script lang="ts">
	let { size = 14 }: { size?: number } = $props();
</script>

<svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">
	<path d="M10 18h10"></path>
	<path d="m17 21 3-3-3-3"></path>
	<path d="M3 11h.01"></path>
	<rect x="15" y="3" width="5" height="8" rx="2.5"></rect>
	<rect x="6" y="3" width="5" height="8" rx="2.5"></rect>
</svg>
