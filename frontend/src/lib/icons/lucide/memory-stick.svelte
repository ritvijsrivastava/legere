<script lang="ts">
	let { size = 14 }: { size?: number } = $props();
</script>

<svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">
	<path d="M12 12v-2"></path>
	<path d="M12 18v-2"></path>
	<path d="M16 12v-2"></path>
	<path d="M16 18v-2"></path>
	<path d="M2 11h1.5"></path>
	<path d="M20 18v-2"></path>
	<path d="M20.5 11H22"></path>
	<path d="M4 18v-2"></path>
	<path d="M8 12v-2"></path>
	<path d="M8 18v-2"></path>
	<rect x="2" y="6" width="20" height="10" rx="2"></rect>
</svg>
