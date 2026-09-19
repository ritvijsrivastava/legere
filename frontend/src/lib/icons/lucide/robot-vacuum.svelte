<script lang="ts">
	let { size = 14 }: { size?: number } = $props();
</script>

<svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">
	<path d="M11 17h2"></path>
	<path d="M12 12h.01"></path>
	<path d="M17 12a5 5 0 00-10 0"></path>
	<path d="M19 2v2.8"></path>
	<path d="M2 5h2.8"></path>
	<path d="M22 5h-2.8"></path>
	<path d="M5 2v2.8"></path>
	<circle cx="12" cy="12" r="10"></circle>
</svg>
