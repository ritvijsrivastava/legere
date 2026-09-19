<script lang="ts">
	let { size = 14 }: { size?: number } = $props();
</script>

<svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">
	<path d="M10 21v-3a2 2 0 014 0v3"></path>
	<path d="M12 2v2"></path>
	<path d="M18 12v9"></path>
	<path d="M22 19a2 2 0 01-2 2H4a2 2 0 01-2-2v-6a1 1 0 011-1h18a1 1 0 011 1z"></path>
	<path d="M4 12a8 8 0 0116 0"></path>
	<path d="M6 12v9"></path>
</svg>
