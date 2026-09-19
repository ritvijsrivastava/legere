<script lang="ts">
	let { size = 14 }: { size?: number } = $props();
</script>

<svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">
	<rect width="18" height="18" x="3" y="3" rx="2"></rect>
	<path d="M12 12H9.5a2.5 2.5 0 0 1 0-5H17"></path>
	<path d="M12 7v10"></path>
	<path d="M16 7v10"></path>
</svg>
