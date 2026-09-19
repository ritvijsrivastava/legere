<script lang="ts">
	let { size = 14 }: { size?: number } = $props();
</script>

<svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">
	<path d="M3 3h18"></path>
	<path d="M20 7H8"></path>
	<path d="M20 11H8"></path>
	<path d="M10 19h10"></path>
	<path d="M8 15h12"></path>
	<path d="M4 3v14"></path>
	<circle cx="4" cy="19" r="2"></circle>
</svg>
