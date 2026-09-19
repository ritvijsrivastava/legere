<script lang="ts">
	let { size = 14 }: { size?: number } = $props();
</script>

<svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">
	<circle cx="7" cy="7" r="5"></circle>
	<circle cx="17" cy="17" r="5"></circle>
	<path d="M12 17h10"></path>
	<path d="m3.46 10.54 7.08-7.08"></path>
</svg>
