<script lang="ts">
	let { size = 14 }: { size?: number } = $props();
</script>

<svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">
	<path d="M9 18V5l11-2v13"></path>
	<circle cx="6.5" cy="18" r="2.5"></circle>
	<circle cx="17.5" cy="16" r="2.5"></circle>
</svg>
