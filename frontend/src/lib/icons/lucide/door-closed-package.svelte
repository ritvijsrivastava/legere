<script lang="ts">
	let { size = 14 }: { size?: number } = $props();
</script>

<svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">
	<path d="M18 13v3"></path>
	<path d="M19 9V5a2 2 0 00-2-2H7a2 2 0 00-2 2v16"></path>
	<path d="M2 21h8"></path>
	<path d="M9 12h.01"></path>
	<rect x="14" y="13" width="8" height="8" rx="1"></rect>
</svg>
