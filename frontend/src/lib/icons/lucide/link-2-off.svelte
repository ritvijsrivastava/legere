<script lang="ts">
	let { size = 14 }: { size?: number } = $props();
</script>

<svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">
	<path d="M9 17H7A5 5 0 0 1 7 7"></path>
	<path d="M15 7h2a5 5 0 0 1 4 8"></path>
	<line x1="8" x2="12" y1="12" y2="12"></line>
	<line x1="2" x2="22" y1="2" y2="22"></line>
</svg>
