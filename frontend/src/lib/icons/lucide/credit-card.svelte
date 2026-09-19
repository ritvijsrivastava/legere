<script lang="ts">
	let { size = 14 }: { size?: number } = $props();
</script>

<svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">
	<rect width="20" height="14" x="2" y="5" rx="2"></rect>
	<line x1="2" x2="22" y1="10" y2="10"></line>
	<path d="M6 14h2"></path>
</svg>
