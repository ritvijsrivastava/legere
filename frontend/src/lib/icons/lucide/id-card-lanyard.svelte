<script lang="ts">
	let { size = 14 }: { size?: number } = $props();
</script>

<svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">
	<path d="M13.5 8h-3"></path>
	<path d="m15 2-1 2h3a2 2 0 012 2v14a2 2 0 01-2 2H7a2 2 0 01-2-2V6a2 2 0 012-2h3"></path>
	<path d="M16 22a4 4 0 00-8 0"></path>
	<path d="m9 2 3 6"></path>
	<circle cx="12" cy="15" r="3"></circle>
</svg>
