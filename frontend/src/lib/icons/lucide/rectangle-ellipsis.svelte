<script lang="ts">
	let { size = 14 }: { size?: number } = $props();
</script>

<svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">
	<rect width="20" height="12" x="2" y="6" rx="2"></rect>
	<path d="M12 12h.01"></path>
	<path d="M17 12h.01"></path>
	<path d="M7 12h.01"></path>
</svg>
