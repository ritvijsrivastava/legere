<script lang="ts">
	let { size = 14 }: { size?: number } = $props();
</script>

<svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">
	<rect width="20" height="16" x="2" y="4" rx="2"></rect>
	<path d="M2 8h20"></path>
	<circle cx="8" cy="14" r="2"></circle>
	<path d="M8 12h8"></path>
	<circle cx="16" cy="14" r="2"></circle>
</svg>
