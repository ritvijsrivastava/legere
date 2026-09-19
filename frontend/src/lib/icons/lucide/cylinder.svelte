<script lang="ts">
	let { size = 14 }: { size?: number } = $props();
</script>

<svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">
	<ellipse cx="12" cy="5" rx="9" ry="3"></ellipse>
	<path d="M3 5v14a9 3 0 0 0 18 0V5"></path>
</svg>
