<script lang="ts">
	let { size = 14 }: { size?: number } = $props();
</script>

<svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">
	<path d="M10 20v2"></path>
	<path d="M14 20v2"></path>
	<path d="M18 20v2"></path>
	<path d="M21 20H3"></path>
	<path d="M6 20v2"></path>
	<path d="M8 16V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v12"></path>
	<rect x="4" y="6" width="16" height="10" rx="2"></rect>
</svg>
