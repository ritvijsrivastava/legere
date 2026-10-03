<script lang="ts">
	import { remoteSyncStore } from '$lib/stores/remoteSync.svelte';
	import * as api from '$lib/api';
	import type { RemoteSyncConfig } from '$lib/types';

	let {
		open,
		initial,
		onclose,
		onSaved
	}: {
		open: boolean;
		/** `null` for first-time setup; an existing config to edit otherwise. */
		initial: RemoteSyncConfig | null;
		onclose: () => void;
		onSaved: (config: RemoteSyncConfig) => void;
	} = $props();

	let endpoint = $state('');
	let bucketName = $state('');
	let region = $state('auto');
	let usePathStyle = $state(false);
	let accessKey = $state('');
	let secretKey = $state('');

	let testState = $state<'idle' | 'testing' | 'ok' | 'failed'>('idle');
	let testError = $state('');
	let saving = $state(false);
	let saveError = $state('');
	let retryingEncryption = $state(false);
	let inputEl = $state<HTMLInputElement>();

	$effect(() => {
		if (!open) return;
		endpoint = initial?.endpoint ?? '';
		bucketName = initial?.bucket_name ?? '';
		region = initial?.region ?? 'auto';
		usePathStyle = initial?.use_path_style ?? false;
		accessKey = initial?.access_key ?? '';
		secretKey = initial?.secret_key ?? '';
		testState = 'idle';
		testError = '';
		saveError = '';
		inputEl?.focus();
	});

	/** Invalidates whatever the last "Test connection" result was the
	 *  moment any bucket-identifying field changes — `save` now trusts
	 *  `testState === 'ok'` directly instead of the backend re-probing the
	 *  bucket on every save (see `draftConfig` below), so a stale 'ok'
	 *  surviving an edit would wrongly mark unverified credentials as
	 *  verified. Also fires (harmlessly, already 'idle') from the
	 *  open-effect above assigning these same fields on open/reset. */
	$effect(() => {
		endpoint;
		bucketName;
		region;
		usePathStyle;
		accessKey;
		secretKey;
		testState = 'idle';
		testError = '';
	});

	/** `device_id` is always backend-owned (see
	 *  `db::sync_config::save_remote_sync_config`'s doc comment) — whatever's
	 *  sent here is ignored server-side, so the placeholder value is never
	 *  actually persisted. `conditional_writes_verified` is the opposite:
	 *  trusted as given (see that same field's doc comment in
	 *  `db::sync_config::RemoteSyncConfig`) — true only when the "Test
	 *  connection" button already confirmed it for exactly these field
	 *  values (the effect above resets `testState` the instant any of them
	 *  change). `sync_interval_hours` isn't edited from this dialog (see
	 *  the Settings screen's "Cross-device sync" stepper instead); carried
	 *  over from `initial` when editing, or the backend's own default when
	 *  there's no existing value yet. */
	function draftConfig(): RemoteSyncConfig {
		return {
			enabled: true,
			endpoint: endpoint.trim(),
			bucket_name: bucketName.trim(),
			region: region.trim(),
			use_path_style: usePathStyle,
			access_key: accessKey.trim(),
			secret_key: secretKey,
			// Backend-computed, like `device_id` below -- whatever's sent here is
			// ignored server-side on save.
			credentials_encrypted: initial?.credentials_encrypted ?? false,
			device_id: initial?.device_id ?? '',
			conditional_writes_verified: testState === 'ok',
			sync_interval_hours: initial?.sync_interval_hours ?? 6
		};
	}

	function isComplete(): boolean {
		return Boolean(endpoint.trim() && bucketName.trim() && region.trim() && accessKey.trim() && secretKey);
	}

	async function testConnection() {
		if (!isComplete()) return;
		testState = 'testing';
		testError = '';
		try {
			const ok = await remoteSyncStore.testConnection(draftConfig());
			testState = ok ? 'ok' : 'failed';
			if (!ok) {
				testError =
					"This provider doesn't support conditional writes, which sync requires to avoid two devices silently overwriting each other.";
			}
		} catch (e) {
			testState = 'failed';
			testError = api.errorMessage(e);
		}
	}

	async function save() {
		if (!isComplete() || saving) return;
		saving = true;
		saveError = '';
		try {
			const saved = await remoteSyncStore.save(draftConfig());
			onSaved(saved);
			close();
		} catch (e) {
			saveError = api.errorMessage(e);
		} finally {
			saving = false;
		}
	}

	function close() {
		if (saving) return;
		onclose();
	}

	async function retryEncryption() {
		if (retryingEncryption) return;
		retryingEncryption = true;
		try {
			await remoteSyncStore.retryCredentialEncryption();
		} finally {
			retryingEncryption = false;
		}
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
			aria-labelledby="remote-sync-title"
			tabindex="-1"
			onclick={(e) => e.stopPropagation()}
			onkeydown={(e) => e.stopPropagation()}
		>
			<div class="dialog-title" id="remote-sync-title">Set up cross-device sync</div>
			<div class="dialog-body">
				Bring your own S3-compatible storage (Cloudflare R2, AWS S3, Backblaze B2, Minio, ...) to
				sync your library across devices you set up yourself. Legere never sees your credentials
				or content — they go straight from this device to your own bucket.
			</div>

			<div class="field">
				<label for="rsync-endpoint">Endpoint URL</label>
				<input
					id="rsync-endpoint"
					class="input"
					type="text"
					placeholder="https://<accountid>.r2.cloudflarestorage.com"
					autocomplete="off"
					spellcheck="false"
					bind:this={inputEl}
					bind:value={endpoint}
				/>
			</div>
			<div class="field">
				<label for="rsync-bucket">Bucket name</label>
				<input
					id="rsync-bucket"
					class="input"
					type="text"
					placeholder="legere-sync"
					autocomplete="off"
					spellcheck="false"
					bind:value={bucketName}
				/>
			</div>
			<div class="field">
				<label for="rsync-region">Region</label>
				<input
					id="rsync-region"
					class="input"
					type="text"
					placeholder="auto"
					autocomplete="off"
					spellcheck="false"
					bind:value={region}
				/>
			</div>
			<div class="field">
				<label for="rsync-access-key">Access key ID</label>
				<input
					id="rsync-access-key"
					class="input"
					type="text"
					autocomplete="off"
					spellcheck="false"
					bind:value={accessKey}
				/>
			</div>
			<div class="field">
				<label for="rsync-secret-key">Secret access key</label>
				<input
					id="rsync-secret-key"
					class="input"
					type="password"
					autocomplete="off"
					spellcheck="false"
					bind:value={secretKey}
				/>
			</div>

			<label class="checkbox-row">
				<input type="checkbox" bind:checked={usePathStyle} />
				<span>Use path-style URLs (most self-hosted providers; leave off for R2/AWS)</span>
			</label>

			<div class="test-row">
				<button
					class="btn btn-secondary"
					onclick={testConnection}
					disabled={!isComplete() || testState === 'testing'}
				>
					{testState === 'testing' ? 'Testing…' : 'Test connection'}
				</button>
				{#if testState === 'ok'}
					<span class="test-ok">Looks good — conditional writes are supported.</span>
				{/if}
			</div>
			{#if testState === 'failed'}
				<div class="dialog-body dialog-body-error">{testError}</div>
			{/if}

			{#if saveError}
				<div class="dialog-body dialog-body-error">{saveError}</div>
			{/if}

			{#if initial && !initial.credentials_encrypted}
				<div class="credential-storage-warning">
					<span
						>Stored unencrypted on this device -- no system keyring/Keystore was available
						when this was last saved.</span
					>
					<button class="btn btn-secondary" onclick={retryEncryption} disabled={retryingEncryption}>
						{retryingEncryption ? 'Retrying...' : 'Retry'}
					</button>
				</div>
			{/if}

			<div class="dialog-actions">
				<button class="btn btn-secondary" onclick={close} disabled={saving}>Cancel</button>
				<button class="btn btn-primary" onclick={save} disabled={!isComplete() || saving}>
					{saving ? 'Saving…' : 'Save & enable'}
				</button>
			</div>
		</div>
	</div>
{/if}

<style>
	.test-row {
		display: flex;
		align-items: center;
		gap: 12px;
	}
	.checkbox-row {
		display: flex;
		align-items: flex-start;
		gap: 8px;
		font-size: 13px;
		color: var(--color-muted);
		cursor: pointer;
	}
	.checkbox-row input {
		margin-top: 2px;
	}
	.test-ok {
		font-size: 13px;
		color: var(--color-accent);
	}
	.credential-storage-warning {
		display: flex;
		align-items: center;
		gap: 12px;
		font-size: 13px;
		color: var(--color-muted);
	}
</style>
