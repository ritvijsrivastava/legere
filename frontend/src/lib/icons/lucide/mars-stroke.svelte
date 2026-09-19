<script lang="ts">
	let { size = 14 }: { size?: number } = $props();
</script>

<svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">
	<path d="m14 6 4 4"></path>
	<path d="M17 3h4v4"></path>
	<path d="m21 3-7.75 7.75"></path>
	<circle cx="9" cy="15" r="6"></circle>
</svg>
