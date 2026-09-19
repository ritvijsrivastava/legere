<script lang="ts">
	let { size = 14 }: { size?: number } = $props();
</script>

<svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">
	<rect width="6" height="16" x="4" y="6" rx="2"></rect>
	<rect width="6" height="9" x="14" y="6" rx="2"></rect>
	<path d="M22 2H2"></path>
</svg>
