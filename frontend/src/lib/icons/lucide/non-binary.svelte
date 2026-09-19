<script lang="ts">
	let { size = 14 }: { size?: number } = $props();
</script>

<svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">
	<path d="M12 2v10"></path>
	<path d="m8.5 4 7 4"></path>
	<path d="m8.5 8 7-4"></path>
	<circle cx="12" cy="17" r="5"></circle>
</svg>
