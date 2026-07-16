<script lang="ts">
	let { size = 15 }: { size?: number } = $props();
</script>

<svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">
	<line x1="12" y1="5" x2="12" y2="19"></line>
	<line x1="5" y1="12" x2="19" y2="12"></line>
</svg>
