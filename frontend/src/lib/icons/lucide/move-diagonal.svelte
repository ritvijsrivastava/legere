<script lang="ts">
	let { size = 14 }: { size?: number } = $props();
</script>

<svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">
	<path d="M11 19H5v-6"></path>
	<path d="M13 5h6v6"></path>
	<path d="M19 5 5 19"></path>
</svg>
