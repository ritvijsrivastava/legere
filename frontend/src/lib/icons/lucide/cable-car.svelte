<script lang="ts">
	let { size = 14 }: { size?: number } = $props();
</script>

<svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">
	<path d="M10 3h.01"></path>
	<path d="M14 2h.01"></path>
	<path d="m2 9 20-5"></path>
	<path d="M12 12V6.5"></path>
	<rect width="16" height="10" x="4" y="12" rx="3"></rect>
	<path d="M9 12v5"></path>
	<path d="M15 12v5"></path>
	<path d="M4 17h16"></path>
</svg>
