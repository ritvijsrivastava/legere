<script lang="ts">
	let { size = 14 }: { size?: number } = $props();
</script>

<svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">
	<circle cx="12" cy="12" r="4"></circle>
	<path d="M12 3v1"></path>
	<path d="M12 20v1"></path>
	<path d="M3 12h1"></path>
	<path d="M20 12h1"></path>
	<path d="m18.364 5.636-.707.707"></path>
	<path d="m6.343 17.657-.707.707"></path>
	<path d="m5.636 5.636.707.707"></path>
	<path d="m17.657 17.657.707.707"></path>
</svg>
