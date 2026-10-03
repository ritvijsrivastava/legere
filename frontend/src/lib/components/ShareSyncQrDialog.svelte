<script lang="ts">
	import * as api from '$lib/api';
	import type { DeviceLinkCode } from '$lib/types';

	let {
		open,
		onclose
	}: {
		open: boolean;
		onclose: () => void;
	} = $props();

	let step = $state<'loading' | 'ready' | 'error'>('loading');
	let code = $state<DeviceLinkCode | null>(null);
	let error = $state('');

	/** Generates a fresh QR/passphrase pair every time the dialog opens —
	 *  each one is meant to be scanned once, right away, not reused later
	 *  or left visible indefinitely. */
	async function generate() {
		step = 'loading';
		error = '';
		code = null;
		try {
			code = await api.generateSyncQr();
			step = 'ready';
		} catch (e) {
			error = api.errorMessage(e);
			step = 'error';
		}
	}

	$effect(() => {
		if (open) generate();
	});
</script>

<svelte:window
	onkeydown={(e) => {
		if (open && e.key === 'Escape') onclose();
	}}
/>

{#if open}
	<div class="dialog-backdrop" role="presentation" onclick={onclose}>
		<div
			class="dialog"
			role="dialog"
			aria-modal="true"
			aria-labelledby="share-sync-qr-title"
			tabindex="-1"
			onclick={(e) => e.stopPropagation()}
			onkeydown={(e) => e.stopPropagation()}
		>
			<div class="dialog-title" id="share-sync-qr-title">Share setup with another device</div>
			<div class="dialog-body">
				On the other device, open Settings → Cross-device sync → Scan from another device, then
				scan this code and enter the passphrase below. Your credentials travel directly between
				these two devices — Legere never sees them.
			</div>

			{#if step === 'loading'}
				<p class="text-muted">Generating code…</p>
			{:else if step === 'error'}
				<div class="dialog-body dialog-body-error">{error}</div>
			{:else if code}
				<div class="qr-wrap">
					<img
						class="qr-image"
						src={`data:image/png;base64,${code.image_base64}`}
						alt="Scannable cross-device sync setup code"
					/>
				</div>
				<div class="passphrase-wrap">
					<span class="passphrase-label">Passphrase</span>
					<span class="passphrase-value tabular-nums">{code.passphrase}</span>
				</div>
				<p class="dialog-body">
					This code is single-use — close this dialog once the other device is set up, and
					generate a new one if you need it again.
				</p>
			{/if}

			<div class="dialog-actions">
				{#if step === 'error'}
					<button class="btn btn-secondary" onclick={generate}>Try again</button>
				{:else}
					<button class="btn btn-secondary" onclick={generate} disabled={step === 'loading'}>
						Regenerate
					</button>
				{/if}
				<button class="btn btn-primary" onclick={onclose}>Done</button>
			</div>
		</div>
	</div>
{/if}

<style>
	.qr-wrap {
		display: flex;
		justify-content: center;
		padding: var(--space-3);
		background: #fff;
		border-radius: var(--radius-lg);
	}
	.qr-image {
		width: 220px;
		height: 220px;
		image-rendering: pixelated;
	}
	.passphrase-wrap {
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 2px;
	}
	.passphrase-label {
		font-size: 12px;
		color: var(--color-muted);
		text-transform: uppercase;
		letter-spacing: 0.08em;
	}
	.passphrase-value {
		font-family: var(--font-heading);
		font-size: 28px;
		font-weight: 700;
		letter-spacing: 0.12em;
	}
</style>
