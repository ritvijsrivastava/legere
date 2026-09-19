<script lang="ts">
	let { size = 14 }: { size?: number } = $props();
</script>

<svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">
	<path d="M10 15v-3"></path>
	<path d="M14 15v-3"></path>
	<path d="M18 15v-3"></path>
	<path d="M2 8V4"></path>
	<path d="M22 6H2"></path>
	<path d="M22 8V4"></path>
	<path d="M6 15v-3"></path>
	<rect x="2" y="12" width="20" height="8" rx="2"></rect>
</svg>
