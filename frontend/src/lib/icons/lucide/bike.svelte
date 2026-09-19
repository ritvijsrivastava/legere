<script lang="ts">
	let { size = 14 }: { size?: number } = $props();
</script>

<svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">
	<circle cx="18.5" cy="17.5" r="3.5"></circle>
	<circle cx="5.5" cy="17.5" r="3.5"></circle>
	<circle cx="15" cy="5" r="1"></circle>
	<path d="M12 17.5V14l-3-3 4-3 2 3h2"></path>
</svg>
