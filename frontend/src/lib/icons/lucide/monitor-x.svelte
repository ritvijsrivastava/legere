<script lang="ts">
	let { size = 14 }: { size?: number } = $props();
</script>

<svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">
	<path d="m14.5 12.5-5-5"></path>
	<path d="m9.5 12.5 5-5"></path>
	<rect width="20" height="14" x="2" y="3" rx="2"></rect>
	<path d="M12 17v4"></path>
	<path d="M8 21h8"></path>
</svg>
