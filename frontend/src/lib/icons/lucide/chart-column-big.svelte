<script lang="ts">
	let { size = 14 }: { size?: number } = $props();
</script>

<svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">
	<path d="M3 3v16a2 2 0 0 0 2 2h16"></path>
	<rect x="15" y="5" width="4" height="12" rx="1"></rect>
	<rect x="7" y="8" width="4" height="9" rx="1"></rect>
</svg>
