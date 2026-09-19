<script lang="ts">
	let { size = 14 }: { size?: number } = $props();
</script>

<svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">
	<path d="M11 7a16 16 20 0 1 10.98 4.362"></path>
	<path d="M12 12a13 13 0 0 1-8.66 5"></path>
	<path d="M16.83 13.634a16 16 0 0 1-9.267 7.328"></path>
	<path d="M20.66 17A13 13 0 0 0 12 12a13 13 0 0 1 0-10"></path>
	<path d="M8.17 15.366a16 16 0 0 1-1.713-11.69"></path>
	<circle cx="12" cy="12" r="10"></circle>
</svg>
