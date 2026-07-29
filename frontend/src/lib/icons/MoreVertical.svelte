<script lang="ts">
	let { size = 15 }: { size?: number } = $props();
</script>

<svg width={size} height={size} viewBox="0 0 24 24" fill="currentColor">
	<circle cx="12" cy="5" r="1.75"></circle>
	<circle cx="12" cy="12" r="1.75"></circle>
	<circle cx="12" cy="19" r="1.75"></circle>
</svg>
