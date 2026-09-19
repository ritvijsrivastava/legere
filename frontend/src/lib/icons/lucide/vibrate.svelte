<script lang="ts">
	let { size = 14 }: { size?: number } = $props();
</script>

<svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">
	<path d="m2 8 2 2-2 2 2 2-2 2"></path>
	<path d="m22 8-2 2 2 2-2 2 2 2"></path>
	<rect width="8" height="14" x="8" y="5" rx="1"></rect>
</svg>
