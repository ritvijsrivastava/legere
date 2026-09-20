<script lang="ts">
	/** The Legere mark: a gather-spiral. `legere` means both "to read" and
	 *  "to gather, to pick out" — scattered feeds and links (the loose line)
	 *  wind inward and resolve into one solid, readable point. Filled/stroked
	 *  in `currentColor` so it inherits the accent wherever it's placed.
	 *  Set `spinning` to use it as a branded loading indicator — the
	 *  gathering motion doubles naturally as "still gathering". */
	let { size = 18, spinning = false }: { size?: number; spinning?: boolean } = $props();
</script>

<svg
	width={size}
	height={size}
	viewBox="0 0 24 24"
	fill="none"
	aria-hidden="true"
	class:spinning
>
	<path
		d="M19.22,8.32 L19.45,9.52 L19.55,10.70 L19.49,11.85 L19.27,12.96 L18.89,13.99 L18.38,14.94 L17.75,15.79 L17.01,16.51 L16.19,17.11 L15.31,17.56 L14.39,17.87 L13.45,18.04 L12.52,18.06 L11.61,17.94 L10.75,17.69 L9.96,17.32 L9.24,16.85 L8.62,16.29 L8.10,15.65 L7.70,14.96 L7.41,14.23 L7.24,13.48 L7.18,12.73 L7.24,12.00 L7.41,11.30 L7.67,10.66 L8.02,10.07 L8.45,9.56 L8.94,9.13 L9.47,8.79 L10.03,8.54 L10.61,8.39 L11.19,8.33 L11.76,8.35 L12.30,8.47 L12.80,8.66 L13.26,8.91 L13.65,9.23 L13.98,9.59 L14.24,9.98 L14.43,10.40 L14.55,10.82 L14.60,11.25 L14.58,11.66 L14.50,12.05 L14.37,12.41 L14.18,12.73 L13.96,13.00"
		stroke="currentColor"
		stroke-width="2.3"
		stroke-linecap="round"
		stroke-linejoin="round"
		stroke-opacity="0.85"
	/>
	<circle cx="13.96" cy="13.00" r="2.6" fill="currentColor" />
</svg>

<style>
	svg {
		transform-origin: 50% 50%;
	}
	.spinning {
		animation: gather-spin 0.9s linear infinite;
	}
	@keyframes gather-spin {
		to {
			transform: rotate(360deg);
		}
	}
</style>
