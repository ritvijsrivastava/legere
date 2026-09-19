<script lang="ts">
	let { size = 14 }: { size?: number } = $props();
</script>

<svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">
	<path d="M15 4H7"></path>
	<path d="m18 16 3 3-3 3"></path>
	<path d="M3 4v13a2 2 0 0 0 2 2h16"></path>
	<path d="M7 14h7"></path>
	<path d="M7 9h12"></path>
</svg>
