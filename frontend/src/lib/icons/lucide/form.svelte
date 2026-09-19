<script lang="ts">
	let { size = 14 }: { size?: number } = $props();
</script>

<svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">
	<path d="M4 14h6"></path>
	<path d="M4 2h10"></path>
	<rect x="4" y="18" width="16" height="4" rx="1"></rect>
	<rect x="4" y="6" width="16" height="4" rx="1"></rect>
</svg>
