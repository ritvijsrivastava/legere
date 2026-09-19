<script lang="ts">
	let { size = 14 }: { size?: number } = $props();
</script>

<svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">
	<path d="M2 8.5 12 4l10 4.5-10 4.5-10-4.5Z"></path>
	<path d="M6.5 10.5v4.5c0 1.4 2.5 2.5 5.5 2.5s5.5-1.1 5.5-2.5v-4.5"></path>
	<line x1="21" y1="8.5" x2="21" y2="14.5"></line>
</svg>
