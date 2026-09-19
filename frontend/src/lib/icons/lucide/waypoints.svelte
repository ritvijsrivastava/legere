<script lang="ts">
	let { size = 14 }: { size?: number } = $props();
</script>

<svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">
	<path d="m10.586 5.414-5.172 5.172"></path>
	<path d="m18.586 13.414-5.172 5.172"></path>
	<path d="M6 12h12"></path>
	<circle cx="12" cy="20" r="2"></circle>
	<circle cx="12" cy="4" r="2"></circle>
	<circle cx="20" cy="12" r="2"></circle>
	<circle cx="4" cy="12" r="2"></circle>
</svg>
