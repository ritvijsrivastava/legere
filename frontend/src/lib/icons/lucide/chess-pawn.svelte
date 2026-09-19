<script lang="ts">
	let { size = 14 }: { size?: number } = $props();
</script>

<svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">
	<path d="M5 20a2 2 0 0 1 2-2h10a2 2 0 0 1 2 2v1a1 1 0 0 1-1 1H6a1 1 0 0 1-1-1z"></path>
	<path d="m14.5 10 1.5 8"></path>
	<path d="M7 10h10"></path>
	<path d="m8 18 1.5-8"></path>
	<circle cx="12" cy="6" r="4"></circle>
</svg>
