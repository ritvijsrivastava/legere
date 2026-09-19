<script lang="ts">
	let { size = 14 }: { size?: number } = $props();
</script>

<svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">
	<path d="M4 12h8"></path>
	<path d="M4 18V6"></path>
	<path d="M12 18V6"></path>
	<path d="m17 12 3-2v8"></path>
</svg>
