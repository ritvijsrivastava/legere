<script lang="ts">
	let { size = 14 }: { size?: number } = $props();
</script>

<svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">
	<path d="M16 2v3"></path>
	<path d="m2 2 20 20"></path>
	<path d="M21 9h-5.5"></path>
	<path d="M3 9h6"></path>
	<path d="M3.586 3.586A2 2 0 003 5v14a2 2 0 002 2h14a2 2 0 001.414-.586"></path>
	<path d="M8.656 3H19a2 2 0 012 2v10.344"></path>
</svg>
