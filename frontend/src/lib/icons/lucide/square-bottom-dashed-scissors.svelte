<script lang="ts">
	let { size = 14 }: { size?: number } = $props();
</script>

<svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">
	<path d="M14 21h1"></path>
	<path d="m17 17-2.18-2.18"></path>
	<path d="M5 21a2 2 0 01-2-2V5a2 2 0 012-2h14a2 2 0 012 2v14a2 2 0 01-2 2"></path>
	<path d="M9 21h1"></path>
	<path d="M9.56 14.44 17 7"></path>
	<path d="M9.56 9.56 12 12"></path>
	<circle cx="8.5" cy="15.5" r="1.5"></circle>
	<circle cx="8.5" cy="8.5" r="1.5"></circle>
</svg>
