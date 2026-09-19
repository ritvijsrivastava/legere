<script lang="ts">
	let { size = 14 }: { size?: number } = $props();
</script>

<svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">
	<path d="m2 2 8 8"></path>
	<path d="m22 2-8 8"></path>
	<ellipse cx="12" cy="9" rx="10" ry="5"></ellipse>
	<path d="M7 13.4v7.9"></path>
	<path d="M12 14v8"></path>
	<path d="M17 13.4v7.9"></path>
	<path d="M2 9v8a10 5 0 0 0 20 0V9"></path>
</svg>
