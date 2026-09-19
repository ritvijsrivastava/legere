<script lang="ts">
	let { size = 14 }: { size?: number } = $props();
</script>

<svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">
	<path d="M13 19a4 4 0 00-8 0"></path>
	<path d="M16 10h2"></path>
	<path d="M16 14h2"></path>
	<circle cx="9" cy="12" r="3"></circle>
	<rect x="2" y="5" width="20" height="14" rx="2"></rect>
</svg>
