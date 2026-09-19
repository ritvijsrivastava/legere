<script lang="ts">
	let { size = 14 }: { size?: number } = $props();
</script>

<svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">
	<path d="M12 12V9a1 1 0 0 0-1-1H9a1 1 0 0 0-1 1v3"></path>
	<path d="M16 20v-3a1 1 0 0 0-1-1h-2a1 1 0 0 0-1 1v3"></path>
	<path d="M20 22V2"></path>
	<path d="M4 12h16"></path>
	<path d="M4 20h16"></path>
	<path d="M4 2v20"></path>
	<path d="M4 4h16"></path>
</svg>
