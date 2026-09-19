<script lang="ts">
	let { size = 14 }: { size?: number } = $props();
</script>

<svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">
	<circle cx="12" cy="12" r="10"></circle>
	<path d="m4.93 4.93 4.24 4.24"></path>
	<path d="m14.83 9.17 4.24-4.24"></path>
	<path d="m14.83 14.83 4.24 4.24"></path>
	<path d="m9.17 14.83-4.24 4.24"></path>
	<circle cx="12" cy="12" r="4"></circle>
</svg>
