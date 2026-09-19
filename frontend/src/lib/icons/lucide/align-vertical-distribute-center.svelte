<script lang="ts">
	let { size = 14 }: { size?: number } = $props();
</script>

<svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">
	<path d="M22 17h-3"></path>
	<path d="M22 7h-5"></path>
	<path d="M5 17H2"></path>
	<path d="M7 7H2"></path>
	<rect x="5" y="14" width="14" height="6" rx="2"></rect>
	<rect x="7" y="4" width="10" height="6" rx="2"></rect>
</svg>
