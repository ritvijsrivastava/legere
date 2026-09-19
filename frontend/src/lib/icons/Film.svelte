<script lang="ts">
	let { size = 14 }: { size?: number } = $props();
</script>

<svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">
	<rect x="3" y="4.5" width="18" height="15" rx="2"></rect>
	<line x1="8" y1="4.5" x2="8" y2="19.5"></line>
	<line x1="16" y1="4.5" x2="16" y2="19.5"></line>
	<line x1="3" y1="9.5" x2="8" y2="9.5"></line>
	<line x1="3" y1="14.5" x2="8" y2="14.5"></line>
	<line x1="16" y1="9.5" x2="21" y2="9.5"></line>
	<line x1="16" y1="14.5" x2="21" y2="14.5"></line>
</svg>
