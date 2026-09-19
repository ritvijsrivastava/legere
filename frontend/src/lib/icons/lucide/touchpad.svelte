<script lang="ts">
	let { size = 14 }: { size?: number } = $props();
</script>

<svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">
	<rect width="20" height="16" x="2" y="4" rx="2"></rect>
	<path d="M2 14h20"></path>
	<path d="M12 20v-6"></path>
</svg>
