<script lang="ts">
	let { size = 14 }: { size?: number } = $props();
</script>

<svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">
	<path d="M5 9h11v6a4 4 0 0 1-4 4H9a4 4 0 0 1-4-4Z"></path>
	<path d="M16 10.5h1.5a2.5 2.5 0 0 1 0 5H16"></path>
	<line x1="7.5" y1="4" x2="7.5" y2="6.5"></line>
	<line x1="10.5" y1="4" x2="10.5" y2="6.5"></line>
</svg>
