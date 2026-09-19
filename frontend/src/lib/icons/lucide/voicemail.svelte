<script lang="ts">
	let { size = 14 }: { size?: number } = $props();
</script>

<svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">
	<circle cx="6" cy="12" r="4"></circle>
	<circle cx="18" cy="12" r="4"></circle>
	<line x1="6" x2="18" y1="16" y2="16"></line>
</svg>
