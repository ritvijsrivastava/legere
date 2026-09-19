<script lang="ts">
	let { size = 14 }: { size?: number } = $props();
</script>

<svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">
	<path d="M16 2v2"></path>
	<path d="M7 21v-2a2 2 0 012-2h6a2 2 0 012 2v2"></path>
	<path d="M8 2v2"></path>
	<circle cx="12" cy="10" r="3"></circle>
	<rect x="3" y="3" width="18" height="18" rx="2"></rect>
</svg>
