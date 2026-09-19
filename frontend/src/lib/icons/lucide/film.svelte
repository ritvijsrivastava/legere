<script lang="ts">
	let { size = 14 }: { size?: number } = $props();
</script>

<svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">
	<rect width="18" height="18" x="3" y="3" rx="2"></rect>
	<path d="M7 3v18"></path>
	<path d="M3 7.5h4"></path>
	<path d="M3 12h18"></path>
	<path d="M3 16.5h4"></path>
	<path d="M17 3v18"></path>
	<path d="M17 7.5h4"></path>
	<path d="M17 16.5h4"></path>
</svg>
