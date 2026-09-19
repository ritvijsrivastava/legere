<script lang="ts">
	let { size = 14 }: { size?: number } = $props();
</script>

<svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">
	<path d="M17.055 4.533a24 24 0 00-10.11 0"></path>
	<path d="M19.467 17.055a24 24 0 000-10.11"></path>
	<path d="M4.533 6.945a24 24 0 000 10.11"></path>
	<path d="M6.945 19.467a24 24 0 0010.11 0"></path>
	<circle cx="19" cy="19" r="2"></circle>
	<circle cx="19" cy="5" r="2"></circle>
	<circle cx="5" cy="19" r="2"></circle>
	<circle cx="5" cy="5" r="2"></circle>
</svg>
