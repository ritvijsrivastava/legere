<script lang="ts">
	let { size = 14 }: { size?: number } = $props();
</script>

<svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">
	<circle cx="10" cy="8" r="5"></circle>
	<path d="M2 21a8 8 0 0 1 10.434-7.62"></path>
	<circle cx="18" cy="18" r="3"></circle>
	<path d="m22 22-1.9-1.9"></path>
</svg>
