<script lang="ts">
	let { size = 14 }: { size?: number } = $props();
</script>

<svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">
	<path d="M2 12a10 10 0 1 1 10 10"></path>
	<path d="m2 22 10-10"></path>
	<path d="M8 22H2v-6"></path>
</svg>
