<script lang="ts">
	let { size = 14 }: { size?: number } = $props();
</script>

<svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">
	<path d="M14 13h4"></path>
	<path d="M16 15v-4"></path>
	<path d="M18 5v2"></path>
	<path d="M6 13h4"></path>
	<path d="M6 5v2"></path>
	<rect x="2" y="7" width="20" height="12" rx="2"></rect>
</svg>
