<script lang="ts">
	let { size = 17 }: { size?: number } = $props();
</script>

<svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">
	<line x1="4" y1="6" x2="20" y2="6"></line>
	<circle cx="9" cy="6" r="2"></circle>
	<line x1="4" y1="12" x2="20" y2="12"></line>
	<circle cx="16" cy="12" r="2"></circle>
	<line x1="4" y1="18" x2="20" y2="18"></line>
	<circle cx="11" cy="18" r="2"></circle>
</svg>
