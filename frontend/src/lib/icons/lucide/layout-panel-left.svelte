<script lang="ts">
	let { size = 14 }: { size?: number } = $props();
</script>

<svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">
	<rect width="7" height="18" x="3" y="3" rx="1"></rect>
	<rect width="7" height="7" x="14" y="3" rx="1"></rect>
	<rect width="7" height="7" x="14" y="14" rx="1"></rect>
</svg>
