<script lang="ts">
	let { size = 14 }: { size?: number } = $props();
</script>

<svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">
	<line x1="10" x2="14" y1="2" y2="2"></line>
	<line x1="12" x2="15" y1="14" y2="11"></line>
	<circle cx="12" cy="14" r="8"></circle>
</svg>
