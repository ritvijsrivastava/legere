<script lang="ts">
	let { size = 14 }: { size?: number } = $props();
</script>

<svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">
	<rect width="18" height="18" x="3" y="3" rx="2"></rect>
	<line x1="12" x2="12" y1="8" y2="12"></line>
	<line x1="12" x2="12.01" y1="16" y2="16"></line>
</svg>
