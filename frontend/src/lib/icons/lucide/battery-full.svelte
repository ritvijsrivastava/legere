<script lang="ts">
	let { size = 14 }: { size?: number } = $props();
</script>

<svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">
	<path d="M10 10v4"></path>
	<path d="M14 10v4"></path>
	<path d="M22 14v-4"></path>
	<path d="M6 10v4"></path>
	<rect x="2" y="6" width="16" height="12" rx="2"></rect>
</svg>
