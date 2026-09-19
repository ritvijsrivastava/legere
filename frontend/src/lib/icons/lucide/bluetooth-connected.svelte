<script lang="ts">
	let { size = 14 }: { size?: number } = $props();
</script>

<svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">
	<path d="m7 7 10 10-5 5V2l5 5L7 17"></path>
	<line x1="18" x2="21" y1="12" y2="12"></line>
	<line x1="3" x2="6" y1="12" y2="12"></line>
</svg>
