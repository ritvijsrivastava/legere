<script lang="ts">
	let { size = 14 }: { size?: number } = $props();
</script>

<svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">
	<path d="M6 3v7a2 2 0 0 0 2 2v9"></path>
	<line x1="6" y1="3" x2="6" y2="8.5"></line>
	<line x1="9" y1="3" x2="9" y2="8.5"></line>
	<path d="M17 3c-1.7 0-3 2-3 5s1.3 5 3 5v8"></path>
</svg>
