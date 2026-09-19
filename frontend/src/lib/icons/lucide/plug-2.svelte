<script lang="ts">
	let { size = 14 }: { size?: number } = $props();
</script>

<svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">
	<path d="M9 2v6"></path>
	<path d="M15 2v6"></path>
	<path d="M12 17v5"></path>
	<path d="M5 8h14"></path>
	<path d="M6 11V8h12v3a6 6 0 1 1-12 0Z"></path>
</svg>
