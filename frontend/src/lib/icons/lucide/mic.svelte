<script lang="ts">
	let { size = 14 }: { size?: number } = $props();
</script>

<svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">
	<path d="M12 19v3"></path>
	<path d="M19 10v2a7 7 0 0 1-14 0v-2"></path>
	<rect x="9" y="2" width="6" height="13" rx="3"></rect>
</svg>
