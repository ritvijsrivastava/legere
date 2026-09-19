<script lang="ts">
	let { size = 14 }: { size?: number } = $props();
</script>

<svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">
	<path d="M14 12h2v8"></path>
	<path d="M14 20h4"></path>
	<path d="M6 12h4"></path>
	<path d="M6 20h4"></path>
	<path d="M8 20V8a4 4 0 0 1 7.464-2"></path>
</svg>
