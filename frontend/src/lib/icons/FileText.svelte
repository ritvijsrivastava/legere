<script lang="ts">
	let { size = 15 }: { size?: number } = $props();
</script>

<svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">
	<path d="M6 2.5h8l5 5v14a1 1 0 0 1-1 1H6a1 1 0 0 1-1-1v-18a1 1 0 0 1 1-1Z"></path>
	<path d="M14 2.5v5h5"></path>
	<path d="M8 13h8"></path>
	<path d="M8 17h8"></path>
	<path d="M8 9h3"></path>
</svg>
