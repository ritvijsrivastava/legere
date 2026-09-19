<script lang="ts">
	let { size = 14 }: { size?: number } = $props();
</script>

<svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">
	<path d="M22 12A10 10 0 1 1 12 2"></path>
	<path d="M22 2 12 12"></path>
	<path d="M16 2h6v6"></path>
</svg>
