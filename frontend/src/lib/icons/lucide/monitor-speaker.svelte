<script lang="ts">
	let { size = 14 }: { size?: number } = $props();
</script>

<svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">
	<path d="M5.5 20H8"></path>
	<path d="M17 9h.01"></path>
	<rect width="10" height="16" x="12" y="4" rx="2"></rect>
	<path d="M8 6H4a2 2 0 0 0-2 2v6a2 2 0 0 0 2 2h4"></path>
	<circle cx="17" cy="15" r="1"></circle>
</svg>
