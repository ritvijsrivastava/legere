<script lang="ts">
	let { size = 14 }: { size?: number } = $props();
</script>

<svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">
	<rect width="20" height="15" x="2" y="4" rx="2"></rect>
	<rect width="8" height="7" x="6" y="8" rx="1"></rect>
	<path d="M18 8v7"></path>
	<path d="M6 19v2"></path>
	<path d="M18 19v2"></path>
</svg>
