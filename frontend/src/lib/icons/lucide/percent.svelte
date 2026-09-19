<script lang="ts">
	let { size = 14 }: { size?: number } = $props();
</script>

<svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">
	<line x1="19" x2="5" y1="5" y2="19"></line>
	<circle cx="6.5" cy="6.5" r="2.5"></circle>
	<circle cx="17.5" cy="17.5" r="2.5"></circle>
</svg>
