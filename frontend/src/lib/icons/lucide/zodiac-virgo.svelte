<script lang="ts">
	let { size = 14 }: { size?: number } = $props();
</script>

<svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">
	<path d="M11 5.5a1 1 0 0 1 5 0V16a5 5 0 0 0 5 5"></path>
	<path d="M16 11.5a1 1 0 0 1 5 0V16a5 5 0 0 1-5 5"></path>
	<path d="M6 19V6a3 3 0 0 0-3-3h0"></path>
	<path d="M6 5.5a1 1 0 0 1 5 0V19"></path>
</svg>
