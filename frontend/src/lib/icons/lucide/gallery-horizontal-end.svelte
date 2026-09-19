<script lang="ts">
	let { size = 14 }: { size?: number } = $props();
</script>

<svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">
	<path d="M2 7v10"></path>
	<path d="M6 5v14"></path>
	<rect width="12" height="18" x="10" y="3" rx="2"></rect>
</svg>
