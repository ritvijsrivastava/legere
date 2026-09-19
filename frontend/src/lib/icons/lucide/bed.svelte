<script lang="ts">
	let { size = 14 }: { size?: number } = $props();
</script>

<svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">
	<path d="M2 4v16"></path>
	<path d="M2 8h18a2 2 0 0 1 2 2v10"></path>
	<path d="M2 17h20"></path>
	<path d="M6 8v9"></path>
</svg>
