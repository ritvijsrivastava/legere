<script lang="ts">
	let { size = 14 }: { size?: number } = $props();
</script>

<svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">
	<path d="M15 3h6v6"></path>
	<path d="m21 3-7 7"></path>
	<path d="m3 21 7-7"></path>
	<path d="M9 21H3v-6"></path>
</svg>
