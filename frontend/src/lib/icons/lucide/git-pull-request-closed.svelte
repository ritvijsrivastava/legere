<script lang="ts">
	let { size = 14 }: { size?: number } = $props();
</script>

<svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">
	<path d="m15.5 3.5 5 5"></path>
	<path d="m15.5 8.5 5-5"></path>
	<path d="M18 11.62V15"></path>
	<path d="M6 9v12"></path>
	<circle cx="18" cy="18" r="3"></circle>
	<circle cx="6" cy="6" r="3"></circle>
</svg>
