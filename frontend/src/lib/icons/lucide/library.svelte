<script lang="ts">
	let { size = 14 }: { size?: number } = $props();
</script>

<svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">
	<path d="m16 6 4 14"></path>
	<path d="M12 6v14"></path>
	<path d="M8 8v12"></path>
	<path d="M4 4v16"></path>
</svg>
