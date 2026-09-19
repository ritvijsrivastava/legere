<script lang="ts">
	let { size = 14 }: { size?: number } = $props();
</script>

<svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">
	<circle cx="12" cy="6" r="1"></circle>
	<line x1="5" x2="19" y1="12" y2="12"></line>
	<circle cx="12" cy="18" r="1"></circle>
</svg>
