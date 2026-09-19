<script lang="ts">
	let { size = 14 }: { size?: number } = $props();
</script>

<svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">
	<rect width="18" height="18" x="3" y="3" rx="2"></rect>
	<path d="M9 14v1"></path>
	<path d="M9 19v2"></path>
	<path d="M9 3v2"></path>
	<path d="M9 9v1"></path>
</svg>
