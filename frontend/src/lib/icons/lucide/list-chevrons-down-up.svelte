<script lang="ts">
	let { size = 14 }: { size?: number } = $props();
</script>

<svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">
	<path d="M3 5h8"></path>
	<path d="M3 12h8"></path>
	<path d="M3 19h8"></path>
	<path d="m15 5 3 3 3-3"></path>
	<path d="m15 19 3-3 3 3"></path>
</svg>
