<script lang="ts">
	let { size = 14 }: { size?: number } = $props();
</script>

<svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">
	<path d="M10 17h.01"></path>
	<path d="M10 7v6"></path>
	<path d="M14 6h2a2 2 0 0 1 2 2v8a2 2 0 0 1-2 2h-2"></path>
	<path d="M22 14v-4"></path>
	<path d="M6 18H4a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h2"></path>
</svg>
