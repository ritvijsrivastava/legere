<script lang="ts">
	import type { AppTheme, ReaderMeasure, ReaderLeading } from '$lib/types';

	let {
		fontSize,
		measure,
		leading,
		theme,
		hasOverride,
		onFontSize,
		onMeasure,
		onLeading,
		onTheme,
		onReset
	}: {
		fontSize: number;
		measure: ReaderMeasure;
		leading: ReaderLeading;
		theme: AppTheme;
		/** Whether this article currently overrides any of the settings
		 *  above (font size/width/line height/theme) rather than following
		 *  the global defaults — shows the "Reset" action below. */
		hasOverride: boolean;
		onFontSize: (size: number) => void;
		onMeasure: (measure: ReaderMeasure) => void;
		onLeading: (leading: ReaderLeading) => void;
		onTheme: (theme: AppTheme) => void;
		onReset: () => void;
	} = $props();

	let open = $state(false);
	let rootEl = $state<HTMLElement | null>(null);

	const measureOptions: { value: ReaderMeasure; label: string }[] = [
		{ value: 'narrow', label: 'S' },
		{ value: 'default', label: 'M' },
		{ value: 'wide', label: 'L' }
	];
	const leadingOptions: { value: ReaderLeading; label: string }[] = [
		{ value: 'compact', label: 'C' },
		{ value: 'default', label: 'D' },
		{ value: 'airy', label: 'A' }
	];
	const themeOptions: { value: AppTheme; label: string }[] = [
		{ value: 'light', label: 'Light' },
		{ value: 'dark', label: 'Dark' }
	];

	function handleClickOutside(e: MouseEvent) {
		if (open && rootEl && !rootEl.contains(e.target as Node)) {
			open = false;
		}
	}
</script>

{#snippet stepper()}
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
{/snippet}

{#snippet measureSeg()}
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
{/snippet}

{#snippet leadingSeg()}
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
{/snippet}

{#snippet themeSeg()}
	<div class="seg">
		{#each themeOptions as opt (opt.value)}
			<label class="seg-opt">
				<input
					type="radio"
					name="reader-theme"
					checked={theme === opt.value}
					onchange={() => onTheme(opt.value)}
				/>
				<span>{opt.label}</span>
			</label>
		{/each}
	</div>
{/snippet}

<svelte:window
	onclick={handleClickOutside}
	onkeydown={(e) => {
		if (open && e.key === 'Escape') open = false;
	}}
/>

<!-- One trigger, one popover, on every platform: the previous desktop
     layout laid the size stepper and three separate segmented controls
     out inline in the header, competing with the back button, favorite,
     mark-read, and overflow menu for attention. Typography is a
     preference you set occasionally, not a control you need visible at
     all times — it earns a single quiet trigger. -->
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
				{@render stepper()}
			</div>
			<div class="aa-row">
				<span class="aa-label">Width</span>
				{@render measureSeg()}
			</div>
			<div class="aa-row">
				<span class="aa-label">Line height</span>
				{@render leadingSeg()}
			</div>
			<div class="aa-row">
				<span class="aa-label">Theme</span>
				{@render themeSeg()}
			</div>
			{#if hasOverride}
				<button class="btn btn-ghost reset-btn" onclick={onReset}>
					Reset to global defaults
				</button>
			{/if}
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
		/* Anchored from the *left* of the trigger, not the right — this
		   button is the first item in a left-anchored button cluster (right
		   after the reader's back button), not the last item of a
		   right-anchored one. A `right: 0` anchor here pushed the whole
		   260px-wide popover off the left edge of a phone screen, clipping
		   every row's label and leaving only the bare controls visible. The
		   `min()` width is a second safety net on screens narrower than
		   260px + this padding could otherwise still overflow. */
		left: 0;
		z-index: 5;
		display: flex;
		flex-direction: column;
		gap: var(--space-3);
		width: min(260px, calc(100vw - 32px));
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
	.reset-btn {
		width: 100%;
		justify-content: center;
		padding: 6px 0 0;
		font-size: 12px;
		border-top: 1px solid var(--color-divider);
		border-radius: 0;
	}
</style>
