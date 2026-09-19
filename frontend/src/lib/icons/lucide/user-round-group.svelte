<script lang="ts">
	let { size = 14 }: { size?: number } = $props();
</script>

<svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">
	<path d="M17 21a5 5 0 00-10 0"></path>
	<path d="M22 10.5a3.5 3.5 0 00-5.507-2.868"></path>
	<path d="M7.507 7.632A3.5 3.5 0 002 10.5"></path>
	<circle cx="12" cy="13" r="3"></circle>
	<circle cx="18.5" cy="4.5" r="2.5"></circle>
	<circle cx="5.5" cy="4.5" r="2.5"></circle>
</svg>
