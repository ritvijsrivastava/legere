<script lang="ts">
	let { size = 14 }: { size?: number } = $props();
</script>

<svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">
	<rect width="18" height="18" x="3" y="3" rx="2"></rect>
	<path d="M8 7v7"></path>
	<path d="M12 7v4"></path>
	<path d="M16 7v9"></path>
</svg>
