<script lang="ts">
	let { size = 14 }: { size?: number } = $props();
</script>

<svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">
	<circle cx="12" cy="10" r="8"></circle>
	<circle cx="12" cy="10" r="3"></circle>
	<path d="M7 22h10"></path>
	<path d="M12 22v-4"></path>
</svg>
