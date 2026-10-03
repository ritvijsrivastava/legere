<script lang="ts">
	import jsQR from 'jsqr';
	import * as api from '$lib/api';
	import type { RemoteSyncConfig } from '$lib/types';

	let {
		open,
		onclose,
		onLinked
	}: {
		open: boolean;
		onclose: () => void;
		/** Fired once the scanned config has been decrypted, verified against
		 *  the bucket, and saved — the caller updates its own store from the
		 *  result, same contract as `RemoteSyncDialog`'s `onSaved`. */
		onLinked: (config: RemoteSyncConfig) => void;
	} = $props();

	type Step = 'camera-error' | 'scanning' | 'passphrase' | 'linking' | 'done';

	let step = $state<Step>('scanning');
	let error = $state('');
	let scannedText = $state('');
	let passphrase = $state('');
	let passphraseInput = $state<HTMLInputElement>();

	let videoEl = $state<HTMLVideoElement>();
	let canvasEl: HTMLCanvasElement | undefined;
	let stream: MediaStream | null = null;
	let rafHandle = 0;

	/** Resolves once every track has actually fired `ended`, not just been
	 *  told to stop "" Android's WebKit/WebView camera stack releases the
	 *  hardware asynchronously, and calling `getUserMedia` again before that
	 *  completes intermittently fails with `NotReadableError`/`TrackStartError`
	 *  (camera still held by the previous session) rather than succeeding or
	 *  throwing something scan-dialog code can tell apart "" this raced every
	 *  "Scan again" after a failed passphrase, since that path stops the
	 *  camera and immediately restarts it. */
	function stopCamera(): Promise<void> {
		cancelAnimationFrame(rafHandle);
		const tracks = stream?.getTracks() ?? [];
		stream = null;
		if (tracks.length === 0) return Promise.resolve();
		return Promise.all(
			tracks.map(
				(track) =>
					new Promise<void>((resolve) => {
						if (track.readyState === 'ended') {
							resolve();
							return;
						}
						track.addEventListener('ended', () => resolve(), { once: true });
						track.stop();
					})
			)
		).then(() => undefined);
	}

	/** Retries once after a short delay on a camera-acquisition failure ""
	 *  covers the same hardware-release race as `stopCamera`'s `ended` wait
	 *  for browsers/WebViews that don't fire `ended` reliably on `stop()`,
	 *  rather than failing a legitimate rescan outright. A second straight
	 *  failure is treated as real (permission denied, no camera, etc.). */
	async function startCamera(isRetry = false) {
		step = 'scanning';
		error = '';
		scannedText = '';
		try {
			stream = await navigator.mediaDevices.getUserMedia({
				video: { facingMode: 'environment' }
			});
			if (!videoEl) return;
			videoEl.srcObject = stream;
			await videoEl.play();
			scanFrame();
		} catch (e) {
			if (!isRetry) {
				await new Promise((resolve) => setTimeout(resolve, 400));
				await startCamera(true);
				return;
			}
			step = 'camera-error';
			const reason = e instanceof DOMException ? ` (${e.name})` : '';
			error = `Camera access was denied or is unavailable${reason}. You can still set up sync manually.`;
		}
	}

	function scanFrame() {
		if (step !== 'scanning' || !videoEl || videoEl.readyState !== videoEl.HAVE_ENOUGH_DATA) {
			rafHandle = requestAnimationFrame(scanFrame);
			return;
		}
		canvasEl ??= document.createElement('canvas');
		canvasEl.width = videoEl.videoWidth;
		canvasEl.height = videoEl.videoHeight;
		const ctx = canvasEl.getContext('2d', { willReadFrequently: true });
		if (!ctx) {
			rafHandle = requestAnimationFrame(scanFrame);
			return;
		}
		ctx.drawImage(videoEl, 0, 0, canvasEl.width, canvasEl.height);
		const frame = ctx.getImageData(0, 0, canvasEl.width, canvasEl.height);
		const result = jsQR(frame.data, frame.width, frame.height);
		if (result?.data) {
			stopCamera();
			scannedText = result.data;
			step = 'passphrase';
			queueMicrotask(() => passphraseInput?.focus());
			return;
		}
		rafHandle = requestAnimationFrame(scanFrame);
	}

	async function submitPassphrase() {
		if (passphrase.trim().length !== 6) return;
		step = 'linking';
		error = '';
		try {
			const decoded = await api.decryptSyncQr(scannedText, passphrase.trim());
			const verified = await api.testRemoteSyncConnection(decoded);
			if (!verified) {
				error =
					"This provider doesn't support conditional writes, which sync requires. Set it up manually instead.";
				step = 'passphrase';
				return;
			}
			const saved = await api.saveRemoteSyncConfig({
				...decoded,
				conditional_writes_verified: true
			});
			onLinked(saved);
			step = 'done';
		} catch (e) {
			error = api.errorMessage(e);
			step = 'passphrase';
		}
	}

	async function rescan() {
		passphrase = '';
		await stopCamera();
		await startCamera();
	}

	$effect(() => {
		if (open) {
			startCamera();
		} else {
			stopCamera();
		}
		return () => {
			stopCamera();
		};
	});

	function close() {
		stopCamera();
		onclose();
	}
</script>

<svelte:window
	onkeydown={(e) => {
		if (open && e.key === 'Escape') close();
	}}
/>

{#if open}
	<div class="dialog-backdrop" role="presentation" onclick={close}>
		<div
			class="dialog"
			role="dialog"
			aria-modal="true"
			aria-labelledby="scan-sync-qr-title"
			tabindex="-1"
			onclick={(e) => e.stopPropagation()}
			onkeydown={(e) => e.stopPropagation()}
		>
			<div class="dialog-title" id="scan-sync-qr-title">Scan from another device</div>

			{#if step === 'scanning'}
				<div class="dialog-body">
					Open Settings → Cross-device sync → Share setup with another device on your already-set-up
					device, then point this camera at the code.
				</div>
				<!-- svelte-ignore a11y_media_has_caption -->
				<video bind:this={videoEl} class="scan-video" muted playsinline></video>
			{:else if step === 'camera-error'}
				<div class="dialog-body dialog-body-error">{error}</div>
			{:else if step === 'passphrase'}
				<div class="dialog-body">Code scanned. Enter the 6-digit passphrase shown on the other device.</div>
				<div class="field">
					<label for="scan-passphrase">Passphrase</label>
					<input
						id="scan-passphrase"
						class="input passphrase-input"
						type="text"
						inputmode="numeric"
						maxlength="6"
						autocomplete="off"
						bind:this={passphraseInput}
						bind:value={passphrase}
						onkeydown={(e) => e.key === 'Enter' && submitPassphrase()}
					/>
				</div>
				{#if error}
					<div class="dialog-body dialog-body-error">{error}</div>
				{/if}
			{:else if step === 'linking'}
				<p class="text-muted">Verifying and saving…</p>
			{:else if step === 'done'}
				<div class="dialog-body">Sync is set up and enabled on this device.</div>
			{/if}

			<div class="dialog-actions">
				{#if step === 'passphrase'}
					<button class="btn btn-secondary" onclick={rescan}>Scan again</button>
					<button
						class="btn btn-primary"
						onclick={submitPassphrase}
						disabled={passphrase.trim().length !== 6}
					>
						Continue
					</button>
				{:else if step === 'camera-error'}
					<button class="btn btn-secondary" onclick={close}>Close</button>
				{:else if step === 'done'}
					<button class="btn btn-primary" onclick={close}>Done</button>
				{:else}
					<button class="btn btn-secondary" onclick={close} disabled={step === 'linking'}>
						Cancel
					</button>
				{/if}
			</div>
		</div>
	</div>
{/if}

<style>
	.scan-video {
		width: 100%;
		aspect-ratio: 1 / 1;
		object-fit: cover;
		border-radius: var(--radius-lg);
		background: #000;
	}
	.passphrase-input {
		font-family: var(--font-heading);
		font-size: 24px;
		font-weight: 700;
		letter-spacing: 0.12em;
		text-align: center;
	}
</style>
