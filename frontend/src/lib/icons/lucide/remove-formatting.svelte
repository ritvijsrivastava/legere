<script lang="ts">
	let { size = 14 }: { size?: number } = $props();
</script>

<svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">
	<path d="M4 7V4h16v3"></path>
	<path d="M5 20h6"></path>
	<path d="M13 4 8 20"></path>
	<path d="m15 15 5 5"></path>
	<path d="m20 15-5 5"></path>
</svg>
