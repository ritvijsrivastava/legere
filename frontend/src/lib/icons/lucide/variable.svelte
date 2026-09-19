<script lang="ts">
	let { size = 14 }: { size?: number } = $props();
</script>

<svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">
	<path d="M8 21s-4-3-4-9 4-9 4-9"></path>
	<path d="M16 3s4 3 4 9-4 9-4 9"></path>
	<line x1="15" x2="9" y1="9" y2="15"></line>
	<line x1="9" x2="15" y1="9" y2="15"></line>
</svg>
