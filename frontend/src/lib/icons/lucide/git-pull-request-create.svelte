<script lang="ts">
	let { size = 14 }: { size?: number } = $props();
</script>

<svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">
	<circle cx="6" cy="6" r="3"></circle>
	<path d="M6 9v12"></path>
	<path d="M13 6h3a2 2 0 0 1 2 2v3"></path>
	<path d="M18 15v6"></path>
	<path d="M21 18h-6"></path>
</svg>
