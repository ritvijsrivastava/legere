<script lang="ts">
	let { size = 14 }: { size?: number } = $props();
</script>

<svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">
	<path d="M2 3v18"></path>
	<rect width="12" height="18" x="6" y="3" rx="2"></rect>
	<path d="M22 3v18"></path>
</svg>
