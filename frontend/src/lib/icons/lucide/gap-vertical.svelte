<script lang="ts">
	let { size = 14 }: { size?: number } = $props();
</script>

<svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">
	<path d="M2 12h2"></path>
	<path d="M8 12h2"></path>
	<path d="M14 12h2"></path>
	<path d="M20 12h2"></path>
	<path d="M3 21v-3a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2v3"></path>
	<path d="M3 3v3a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2V3"></path>
</svg>
