<script lang="ts">
	let { size = 14 }: { size?: number } = $props();
</script>

<svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">
	<rect width="18" height="18" x="3" y="3" rx="2"></rect>
	<path d="M12 9v6"></path>
	<path d="M16 15v6"></path>
	<path d="M16 3v6"></path>
	<path d="M3 15h18"></path>
	<path d="M3 9h18"></path>
	<path d="M8 15v6"></path>
	<path d="M8 3v6"></path>
</svg>
