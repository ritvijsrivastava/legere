<script lang="ts">
	let { size = 14 }: { size?: number } = $props();
</script>

<svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">
	<path d="M12 22v-6"></path>
	<path d="M12 8V2"></path>
	<path d="M4 12H2"></path>
	<path d="M10 12H8"></path>
	<path d="M16 12h-2"></path>
	<path d="M22 12h-2"></path>
	<path d="m15 19-3 3-3-3"></path>
	<path d="m15 5-3-3-3 3"></path>
</svg>
