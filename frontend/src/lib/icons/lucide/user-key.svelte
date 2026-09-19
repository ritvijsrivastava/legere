<script lang="ts">
	let { size = 14 }: { size?: number } = $props();
</script>

<svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">
	<path d="M20 11v6"></path>
	<path d="M20 13h2"></path>
	<path d="M3 21v-2a4 4 0 0 1 4-4h6a4 4 0 0 1 2.072.578"></path>
	<circle cx="10" cy="7" r="4"></circle>
	<circle cx="20" cy="19" r="2"></circle>
</svg>
