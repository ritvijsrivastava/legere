<script lang="ts">
	let { size = 16 }: { size?: number } = $props();
</script>

<svg
	width={size}
	height={size}
	viewBox="0 0 24 24"
	fill="none"
	stroke="currentColor"
	stroke-width="1.75"
	stroke-linecap="round"
	stroke-linejoin="round"
	aria-hidden="true"
>
	<circle cx="18" cy="5" r="3"></circle>
	<circle cx="6" cy="12" r="3"></circle>
	<circle cx="18" cy="19" r="3"></circle>
	<path d="m8.6 13.5 6.8 4"></path>
	<path d="m15.4 6.5-6.8 4"></path>
</svg>
