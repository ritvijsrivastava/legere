<script lang="ts">
	// Global dialog (mounted once from the root layout, like
	// `AddSourceDialog`) surfacing whatever `captureJobsStore` currently
	// knows about — the "Add a source" dialog itself closes the instant a
	// job is queued, so this is the only place progress, failure, and
	// retry live once that happens.
	import { uiStore } from '$lib/stores/ui.svelte';
	import { captureJobsStore } from '$lib/stores/captureJobs.svelte';
	import Refresh from '$lib/icons/Refresh.svelte';

	let open = $derived(uiStore.captureJobsOpen);
	let retryingId = $state<string | null>(null);
	let dismissingId = $state<string | null>(null);
	let cancellingId = $state<string | null>(null);
	let busyId = $derived(retryingId ?? dismissingId ?? cancellingId);

	function close() {
		uiStore.closeCaptureJobs();
	}

	async function retry(id: string) {
		retryingId = id;
		try {
			await captureJobsStore.retry(id);
		} finally {
			retryingId = null;
		}
	}

	async function dismiss(id: string) {
		dismissingId = id;
		try {
			await captureJobsStore.dismiss(id);
		} finally {
			dismissingId = null;
		}
	}

	async function cancel(id: string) {
		cancellingId = id;
		try {
			await captureJobsStore.cancel(id);
		} finally {
			cancellingId = null;
		}
	}

	/** A job only ever has the raw URL to identify itself by \u2014 the title
	 *  is only known once capture succeeds, and a succeeded job is never
	 *  in this list (see `CaptureJob`'s doc comment) \u2014 so this trims it
	 *  to something scannable instead of showing a full, often-long URL. */
	function shortUrl(url: string): string {
		try {
			const parsed = new URL(url);
			return parsed.hostname + (parsed.pathname === '/' ? '' : parsed.pathname);
		} catch {
			return url;
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
			aria-labelledby="capture-jobs-title"
			tabindex="-1"
			onclick={(e) => e.stopPropagation()}
			onkeydown={(e) => e.stopPropagation()}
		>
			<div class="dialog-title" id="capture-jobs-title">Adding sources</div>

			{#if captureJobsStore.jobs.length === 0}
				<div class="dialog-body">Nothing in progress.</div>
			{:else}
				<ul class="job-list">
					{#each captureJobsStore.jobs as job (job.id)}
						<li class="job-row">
							<div class="job-info">
								<span class="job-icon" class:job-icon-failed={job.status.state === 'failed'}>
									{#if job.status.state === 'running'}
										<Refresh size={14} spinning />
									{:else}
										!
									{/if}
								</span>
								<div class="job-text">
									<div class="job-url">{shortUrl(job.url)}</div>
									{#if job.status.state === 'running'}
										<div class="job-status text-muted">Fetching and extracting…</div>
									{:else}
										<div class="job-status job-status-error">{job.status.message}</div>
									{/if}
								</div>
							</div>
							{#if job.status.state === 'failed'}
								<div class="job-actions">
									<button
										class="btn btn-secondary"
										onclick={() => dismiss(job.id)}
										disabled={busyId !== null}
									>
										Dismiss
									</button>
									<button
										class="btn btn-primary"
										onclick={() => retry(job.id)}
										disabled={busyId !== null}
									>
										{retryingId === job.id ? 'Retrying…' : 'Retry'}
									</button>
								</div>
							{:else}
								<div class="job-actions">
									<button
										class="btn btn-secondary"
										onclick={() => cancel(job.id)}
										disabled={busyId !== null}
									>
										{cancellingId === job.id ? 'Cancelling…' : 'Cancel'}
									</button>
								</div>
							{/if}
						</li>
					{/each}
				</ul>
			{/if}

			<div class="dialog-actions">
				<button class="btn btn-secondary" onclick={close}>Close</button>
			</div>
		</div>
	</div>
{/if}

<style>
	.job-list {
		list-style: none;
		margin: 0;
		padding: 0;
		display: flex;
		flex-direction: column;
		gap: 10px;
		max-height: 320px;
		overflow-y: auto;
	}
	.job-row {
		display: flex;
		flex-direction: column;
		gap: 10px;
		padding: 10px 12px;
		border-radius: var(--radius-md);
		background: var(--color-surface);
	}
	.job-info {
		display: flex;
		align-items: flex-start;
		gap: 10px;
		min-width: 0;
	}
	.job-icon {
		display: flex;
		align-items: center;
		justify-content: center;
		flex: none;
		width: 22px;
		height: 22px;
		margin-top: 1px;
		border-radius: 50%;
		font-size: 12px;
		font-weight: 700;
		color: var(--color-muted);
		background: var(--color-surface-raised);
	}
	.job-icon-failed {
		color: var(--color-danger);
	}
	.job-text {
		min-width: 0;
		flex: 1;
	}
	.job-url {
		font-size: 13.5px;
		font-weight: 600;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.job-status {
		font-size: 12px;
		margin-top: 2px;
	}
	.job-status-error {
		color: var(--color-danger);
	}
	.job-actions {
		display: flex;
		justify-content: flex-end;
		gap: 8px;
	}
</style>
