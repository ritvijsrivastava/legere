<script lang="ts">
	let { size = 14 }: { size?: number } = $props();
</script>

<svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">
	<path d="M18 21a6 6 0 0 0-12 0"></path>
	<circle cx="12" cy="11" r="4"></circle>
	<rect width="18" height="18" x="3" y="3" rx="2"></rect>
</svg>
