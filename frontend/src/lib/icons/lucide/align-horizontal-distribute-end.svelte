<script lang="ts">
	let { size = 14 }: { size?: number } = $props();
</script>

<svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">
	<rect width="6" height="14" x="4" y="5" rx="2"></rect>
	<rect width="6" height="10" x="14" y="7" rx="2"></rect>
	<path d="M10 2v20"></path>
	<path d="M20 2v20"></path>
</svg>
