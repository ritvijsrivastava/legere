<script lang="ts">
	let { size = 14 }: { size?: number } = $props();
</script>

<svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">
	<path d="M12 2.5c2.8 1.4 4.5 4.4 4.5 8.5c0 2.4-.6 4.3-1.3 5.7L12 19l-3.2-2.3c-.7-1.4-1.3-3.3-1.3-5.7c0-4.1 1.7-7.1 4.5-8.5Z"></path>
	<circle cx="12" cy="10.5" r="1.8"></circle>
	<path d="M8.5 16 6 18.5V21l3-1.6"></path>
	<path d="M15.5 16 18 18.5V21l-3-1.6"></path>
</svg>
