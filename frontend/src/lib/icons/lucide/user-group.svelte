<script lang="ts">
	let { size = 14 }: { size?: number } = $props();
</script>

<svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">
	<path d="M17 21v-1a2 2 0 00-2-2H9a2 2 0 00-2 2v1"></path>
	<path d="M19 10h1a2 2 0 012 2v1"></path>
	<path d="M5 10H4a2 2 0 00-2 2v1"></path>
	<circle cx="12" cy="11" r="3"></circle>
	<circle cx="18" cy="4" r="2"></circle>
	<circle cx="6" cy="4" r="2"></circle>
</svg>
