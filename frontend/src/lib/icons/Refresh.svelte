<script lang="ts">
	let { size = 15, spinning = false }: { size?: number; spinning?: boolean } = $props();
</script>

<svg
	width={size}
	height={size}
	viewBox="0 0 24 24"
	fill="none"
	stroke="currentColor"
	stroke-width="1.75"
	stroke-linecap="round"
	stroke-linejoin="round"
	class:spinning
>
	<path d="M21 12a9 9 0 1 1-2.64-6.36"></path>
	<path d="M21 3v6h-6"></path>
</svg>

<style>
	.spinning {
		animation: spin 0.9s linear infinite;
	}
	@keyframes spin {
		to {
			transform: rotate(360deg);
		}
	}
</style>
