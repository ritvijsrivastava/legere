<script lang="ts">
	let { size = 14 }: { size?: number } = $props();
</script>

<svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">
	<path d="M10 3v11"></path>
	<path d="M10 9H7a1 1 0 0 1 0-6h8"></path>
	<path d="M14 3v11"></path>
	<path d="m18 14 4 4H2"></path>
	<path d="m22 18-4 4"></path>
</svg>
