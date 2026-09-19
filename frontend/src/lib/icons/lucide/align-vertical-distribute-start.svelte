<script lang="ts">
	let { size = 14 }: { size?: number } = $props();
</script>

<svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">
	<rect width="14" height="6" x="5" y="14" rx="2"></rect>
	<rect width="10" height="6" x="7" y="4" rx="2"></rect>
	<path d="M2 14h20"></path>
	<path d="M2 4h20"></path>
</svg>
