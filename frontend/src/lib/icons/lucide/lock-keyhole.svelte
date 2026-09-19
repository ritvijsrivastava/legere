<script lang="ts">
	let { size = 14 }: { size?: number } = $props();
</script>

<svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">
	<circle cx="12" cy="16" r="1"></circle>
	<rect x="3" y="10" width="18" height="12" rx="2"></rect>
	<path d="M7 10V7a5 5 0 0 1 10 0v3"></path>
</svg>
