<script lang="ts">
	let { size = 14 }: { size?: number } = $props();
</script>

<svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">
	<circle cx="19" cy="5" r="2"></circle>
	<circle cx="5" cy="19" r="2"></circle>
	<path d="M5 17A12 12 0 0 1 17 5"></path>
</svg>
