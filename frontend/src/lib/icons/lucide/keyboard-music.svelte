<script lang="ts">
	let { size = 14 }: { size?: number } = $props();
</script>

<svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">
	<rect width="20" height="16" x="2" y="4" rx="2"></rect>
	<path d="M6 8h4"></path>
	<path d="M14 8h.01"></path>
	<path d="M18 8h.01"></path>
	<path d="M2 12h20"></path>
	<path d="M6 12v4"></path>
	<path d="M10 12v4"></path>
	<path d="M14 12v4"></path>
	<path d="M18 12v4"></path>
</svg>
