<script lang="ts">
	let { size = 14 }: { size?: number } = $props();
</script>

<svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">
	<path d="M15 10V9"></path>
	<path d="M9 10V9"></path>
	<path d="M9 16a5 5 0 016 0"></path>
	<circle cx="12" cy="12" r="10"></circle>
</svg>
