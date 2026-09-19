<script lang="ts">
	let { size = 14 }: { size?: number } = $props();
</script>

<svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">
	<rect x="3" y="7.5" width="18" height="12" rx="2"></rect>
	<path d="M8 7.5V6a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v1.5"></path>
	<line x1="3" y1="12.5" x2="21" y2="12.5"></line>
</svg>
