<script lang="ts">
	let { size = 14 }: { size?: number } = $props();
</script>

<svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">
	<rect width="8" height="8" x="3" y="3" rx="2"></rect>
	<path d="M7 11v4a2 2 0 0 0 2 2h4"></path>
	<rect width="8" height="8" x="13" y="13" rx="2"></rect>
</svg>
