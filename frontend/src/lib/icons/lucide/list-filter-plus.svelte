<script lang="ts">
	let { size = 14 }: { size?: number } = $props();
</script>

<svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">
	<path d="M12 5H2"></path>
	<path d="M6 12h12"></path>
	<path d="M9 19h6"></path>
	<path d="M16 5h6"></path>
	<path d="M19 8V2"></path>
</svg>
