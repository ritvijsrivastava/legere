<script lang="ts">
	let { size = 14 }: { size?: number } = $props();
</script>

<svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">
	<circle cx="12" cy="12" r="3"></circle>
	<line x1="3" x2="9" y1="12" y2="12"></line>
	<line x1="15" x2="21" y1="12" y2="12"></line>
</svg>
