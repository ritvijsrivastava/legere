<script lang="ts">
	let { size = 14 }: { size?: number } = $props();
</script>

<svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">
	<path d="M10 15H4a2 2 0 01-2-2V7a2 2 0 012-2h6"></path>
	<path d="M10 19H5"></path>
	<path d="M14 11h8"></path>
	<path d="M14 7h8"></path>
	<path d="M18 17h.01"></path>
	<path d="M9 19v-4"></path>
	<rect x="14" y="3" width="8" height="18" rx="1"></rect>
</svg>
