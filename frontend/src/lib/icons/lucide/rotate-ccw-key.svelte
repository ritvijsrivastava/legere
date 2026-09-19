<script lang="ts">
	let { size = 14 }: { size?: number } = $props();
</script>

<svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">
	<path d="M12 7v6"></path>
	<path d="M12 9h2"></path>
	<path d="M3 12a9 9 0 1 0 9-9 9.74 9.74 0 0 0-6.74 2.74L3 8"></path>
	<path d="M3 3v5h5"></path>
	<circle cx="12" cy="15" r="2"></circle>
</svg>
