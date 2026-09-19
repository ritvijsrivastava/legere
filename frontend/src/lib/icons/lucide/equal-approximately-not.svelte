<script lang="ts">
	let { size = 14 }: { size?: number } = $props();
</script>

<svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">
	<path d="M5 15a6.5 6.5 0 0 1 7 0 6.5 6.5 0 0 0 7 0"></path>
	<path d="M5 9a6.5 6.5 0 0 1 7 0 6.5 6.5 0 0 0 7 0"></path>
	<line x1="19" x2="5" y1="5" y2="19"></line>
</svg>
