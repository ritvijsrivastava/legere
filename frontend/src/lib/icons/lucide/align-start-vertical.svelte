<script lang="ts">
	let { size = 14 }: { size?: number } = $props();
</script>

<svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">
	<rect width="9" height="6" x="6" y="14" rx="2"></rect>
	<rect width="16" height="6" x="6" y="4" rx="2"></rect>
	<path d="M2 2v20"></path>
</svg>
