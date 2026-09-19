<script lang="ts">
	let { size = 14 }: { size?: number } = $props();
</script>

<svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">
	<circle cx="11" cy="11" r="8"></circle>
	<path d="m21 21-4.3-4.3"></path>
	<path d="M11 7v4"></path>
	<path d="M11 15h.01"></path>
</svg>
