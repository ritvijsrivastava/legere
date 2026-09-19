<script lang="ts">
	let { size = 14 }: { size?: number } = $props();
</script>

<svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">
	<path d="M13 5h8"></path>
	<path d="M13 12h8"></path>
	<path d="M13 19h8"></path>
	<path d="m3 17 2 2 4-4"></path>
	<rect x="3" y="4" width="6" height="6" rx="1"></rect>
</svg>
