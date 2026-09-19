<script lang="ts">
	let { size = 14 }: { size?: number } = $props();
</script>

<svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">
	<rect width="20" height="16" x="2" y="4" rx="2"></rect>
	<path d="M12 9v11"></path>
	<path d="M2 9h13a2 2 0 0 1 2 2v9"></path>
</svg>
