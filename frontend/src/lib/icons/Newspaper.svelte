<script lang="ts">
	let { size = 14 }: { size?: number } = $props();
</script>

<svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">
	<path d="M4 5h12a2 2 0 0 1 2 2v12a2 2 0 0 1-2-2V7a2 2 0 0 0-2-2H4Z"></path>
	<path d="M4 5v14h13"></path>
	<line x1="7" y1="9" x2="13" y2="9"></line>
	<line x1="7" y1="13" x2="13" y2="13"></line>
	<line x1="7" y1="17" x2="10" y2="17"></line>
</svg>
