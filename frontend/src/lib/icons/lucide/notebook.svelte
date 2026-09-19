<script lang="ts">
	let { size = 14 }: { size?: number } = $props();
</script>

<svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">
	<path d="M2 6h4"></path>
	<path d="M2 10h4"></path>
	<path d="M2 14h4"></path>
	<path d="M2 18h4"></path>
	<rect width="16" height="20" x="4" y="2" rx="2"></rect>
	<path d="M16 2v20"></path>
</svg>
