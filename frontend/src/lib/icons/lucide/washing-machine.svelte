<script lang="ts">
	let { size = 14 }: { size?: number } = $props();
</script>

<svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">
	<path d="M3 6h3"></path>
	<path d="M17 6h.01"></path>
	<rect width="18" height="20" x="3" y="2" rx="2"></rect>
	<circle cx="12" cy="13" r="5"></circle>
	<path d="M12 18a2.5 2.5 0 0 0 0-5 2.5 2.5 0 0 1 0-5"></path>
</svg>
