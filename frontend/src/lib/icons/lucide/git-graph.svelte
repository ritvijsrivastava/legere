<script lang="ts">
	let { size = 14 }: { size?: number } = $props();
</script>

<svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">
	<circle cx="5" cy="6" r="3"></circle>
	<path d="M5 9v6"></path>
	<circle cx="5" cy="18" r="3"></circle>
	<path d="M12 3v18"></path>
	<circle cx="19" cy="6" r="3"></circle>
	<path d="M16 15.7A9 9 0 0 0 19 9"></path>
</svg>
