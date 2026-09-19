<script lang="ts">
	let { size = 14 }: { size?: number } = $props();
</script>

<svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">
	<path d="M12 17v4"></path>
	<path d="M8 21h8"></path>
	<path d="m9 17 6.1-6.1a2 2 0 0 1 2.81.01L22 15"></path>
	<circle cx="8" cy="9" r="2"></circle>
	<rect x="2" y="3" width="20" height="14" rx="2"></rect>
</svg>
