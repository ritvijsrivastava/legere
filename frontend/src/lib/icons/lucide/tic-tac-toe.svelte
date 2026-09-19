<script lang="ts">
	let { size = 14 }: { size?: number } = $props();
</script>

<svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">
	<path d="M12 2v20"></path>
	<path d="m21 16-5 5"></path>
	<path d="m21 21-5-5"></path>
	<path d="M22 12H2"></path>
	<path d="M8 3 3 8"></path>
	<path d="M8 8 3 3"></path>
	<circle cx="18.5" cy="5.5" r="2.5"></circle>
	<circle cx="5.5" cy="18.5" r="2.5"></circle>
</svg>
