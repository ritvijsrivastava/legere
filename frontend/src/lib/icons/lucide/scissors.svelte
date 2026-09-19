<script lang="ts">
	let { size = 14 }: { size?: number } = $props();
</script>

<svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">
	<circle cx="6" cy="6" r="3"></circle>
	<path d="M8.12 8.12 12 12"></path>
	<path d="M20 4 8.12 15.88"></path>
	<circle cx="6" cy="18" r="3"></circle>
	<path d="M14.8 14.8 20 20"></path>
</svg>
