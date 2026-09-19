<script lang="ts">
	let { size = 14 }: { size?: number } = $props();
</script>

<svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">
	<path d="M13 18H4a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h16a2 2 0 0 1 2 2v5"></path>
	<path d="m17 17 5 5"></path>
	<path d="M18 12h.01"></path>
	<path d="m22 17-5 5"></path>
	<path d="M6 12h.01"></path>
	<circle cx="12" cy="12" r="2"></circle>
</svg>
