<script lang="ts">
	let { size = 14 }: { size?: number } = $props();
</script>

<svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">
	<rect width="7" height="7" x="3" y="3" rx="1"></rect>
	<rect width="7" height="7" x="3" y="14" rx="1"></rect>
	<path d="M18 3v18"></path>
	<path d="m21 18-3 3-3-3"></path>
</svg>
