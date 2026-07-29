<script lang="ts">
	import type { ReaderMeasure, ReaderLeading } from '$lib/types';

	let {
		fontSize,
		measure,
		leading,
		onFontSize,
		onMeasure,
		onLeading
	}: {
		fontSize: number;
		measure: ReaderMeasure;
		leading: ReaderLeading;
		onFontSize: (size: number) => void;
		onMeasure: (measure: ReaderMeasure) => void;
		onLeading: (leading: ReaderLeading) => void;
	} = $props();

	let open = $state(false);
	let rootEl = $state<HTMLElement | null>(null);

	const measureOptions: { value: ReaderMeasure; label: string }[] = [
		{ value: 'narrow', label: 'Narrow' },
		{ value: 'default', label: 'Default' },
		{ value: 'wide', label: 'Wide' }
	];
	const leadingOptions: { value: ReaderLeading; label: string }[] = [
		{ value: 'compact', label: 'Compact' },
		{ value: 'default', label: 'Default' },
		{ value: 'airy', label: 'Airy' }
	];

	function handleClickOutside(e: MouseEvent) {
		if (open && rootEl && !rootEl.contains(e.target as Node)) {
			open = false;
		}
	}
</script>

<svelte:window
	onclick={handleClickOutside}
	onkeydown={(e) => {
		if (open && e.key === 'Escape') open = false;
	}}
/>

<div class="reader-controls" bind:this={rootEl}>
	<button
		class="btn btn-secondary btn-icon aa-trigger"
		onclick={() => (open = !open)}
		aria-label="Typography settings"
		aria-expanded={open}
	>
		Aa
	</button>
	{#if open}
		<div class="aa-popover elev-md" role="dialog" aria-label="Typography settings">
			<div class="aa-row">
				<span class="aa-label">Size</span>
				<div class="aa-stepper">
					<button
						class="btn btn-icon btn-secondary"
						onclick={() => onFontSize(Math.max(16, fontSize - 1))}
						disabled={fontSize <= 16}
						aria-label="Decrease font size"
					>
						–
					</button>
					<span class="aa-value">{fontSize}</span>
					<button
						class="btn btn-icon btn-secondary"
						onclick={() => onFontSize(Math.min(22, fontSize + 1))}
						disabled={fontSize >= 22}
						aria-label="Increase font size"
					>
						+
					</button>
				</div>
			</div>
			<div class="aa-row">
				<span class="aa-label">Width</span>
				<div class="seg">
					{#each measureOptions as opt (opt.value)}
						<label class="seg-opt">
							<input
								type="radio"
								name="reader-measure"
								checked={measure === opt.value}
								onchange={() => onMeasure(opt.value)}
							/>
							<span>{opt.label}</span>
						</label>
					{/each}
				</div>
			</div>
			<div class="aa-row">
				<span class="aa-label">Line height</span>
				<div class="seg">
					{#each leadingOptions as opt (opt.value)}
						<label class="seg-opt">
							<input
								type="radio"
								name="reader-leading"
								checked={leading === opt.value}
								onchange={() => onLeading(opt.value)}
							/>
							<span>{opt.label}</span>
						</label>
					{/each}
				</div>
			</div>
		</div>
	{/if}
</div>

<style>
	.reader-controls {
		position: relative;
	}
	.aa-trigger {
		width: auto;
		padding: 0 var(--space-3);
		font-family: var(--font-heading);
	}
	.aa-popover {
		position: absolute;
		top: calc(100% + 8px);
		right: 0;
		z-index: 5;
		display: flex;
		flex-direction: column;
		gap: var(--space-3);
		width: 260px;
		padding: var(--space-4);
		border-radius: var(--radius-lg);
		background: var(--color-surface);
	}
	.aa-row {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: var(--space-3);
	}
	.aa-label {
		font-size: 12px;
		color: color-mix(in srgb, var(--color-text) 70%, transparent);
	}
	.aa-stepper {
		display: flex;
		align-items: center;
		gap: 6px;
	}
	.aa-value {
		min-width: 20px;
		text-align: center;
		font-size: 13px;
		font-variant-numeric: tabular-nums;
	}
</style>
