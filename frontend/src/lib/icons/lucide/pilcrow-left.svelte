<script lang="ts">
	let { size = 14 }: { size?: number } = $props();
</script>

<svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">
	<path d="M14 3v11"></path>
	<path d="M14 9h-3a3 3 0 0 1 0-6h9"></path>
	<path d="M18 3v11"></path>
	<path d="M22 18H2l4-4"></path>
	<path d="m6 22-4-4"></path>
</svg>
