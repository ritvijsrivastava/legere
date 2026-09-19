<script lang="ts">
	let { size = 14 }: { size?: number } = $props();
</script>

<svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">
	<path d="M4 9V5a2 2 0 0 1 2-2h12a2 2 0 0 1 2 2v4"></path>
	<path d="M8 8v1"></path>
	<path d="M12 8v1"></path>
	<path d="M16 8v1"></path>
	<rect width="20" height="12" x="2" y="9" rx="2"></rect>
	<circle cx="8" cy="15" r="2"></circle>
	<circle cx="16" cy="15" r="2"></circle>
</svg>
