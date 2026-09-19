<script lang="ts">
	let { size = 14 }: { size?: number } = $props();
</script>

<svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">
	<path d="M10 10.01h.01"></path>
	<path d="M10 14.01h.01"></path>
	<path d="M14 10.01h.01"></path>
	<path d="M14 14.01h.01"></path>
	<path d="M18 6v12"></path>
	<path d="M6 6v12"></path>
	<rect x="2" y="6" width="20" height="12" rx="2"></rect>
</svg>
