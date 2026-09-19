<script lang="ts">
	let { size = 14 }: { size?: number } = $props();
</script>

<svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">
	<rect width="20" height="8" x="2" y="2" rx="2" ry="2"></rect>
	<rect width="20" height="8" x="2" y="14" rx="2" ry="2"></rect>
	<line x1="6" x2="6.01" y1="6" y2="6"></line>
	<line x1="6" x2="6.01" y1="18" y2="18"></line>
</svg>
