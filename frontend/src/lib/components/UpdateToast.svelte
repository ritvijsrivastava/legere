<script lang="ts">
	import { getReleaseNotes, parseChangelog } from '$lib/update';

	let {
		version,
		notes,
		onUpdate,
		onDismiss,
		variant = 'floating'
	}: {
		version: string;
		notes?: string | null;
		onUpdate: () => void;
		onDismiss: () => void;
		variant?: 'floating' | 'inline';
	} = $props();

	let expanded = $state(false);
	let detailsState = $state<'idle' | 'loading' | 'loaded' | 'error'>('idle');
	let changes = $state<string[]>([]);

	// `notes` (from checkForUpdate, sourced from latest.json) is stale/empty
	// by the time a user sees it — see getReleaseNotes()'s doc comment — so
	// fetch the real changelog for this version lazily, falling back to the
	// raw notes if that fetch comes up empty or fails.
	async function toggleDetails() {
		expanded = !expanded;
		if (!expanded || detailsState === 'loading' || detailsState === 'loaded') return;
		detailsState = 'loading';
		try {
			const info = await getReleaseNotes(version);
			changes = parseChangelog(info?.notes);
			detailsState = 'loaded';
		} catch {
			detailsState = 'error';
		}
	}
</script>

<div class="update-toast elev-lg" class:inline={variant === 'inline'}>
	<div class="title">Update available — v{version}</div>
	<button class="btn btn-ghost details-btn" onclick={toggleDetails}>
		{expanded ? 'Hide details ▴' : 'Details ▾'}
	</button>
	{#if expanded}
		{#if detailsState === 'loading'}
			<div class="changelog-status text-muted">Loading…</div>
		{:else if changes.length}
			<ul class="changelog text-muted">
				{#each changes as change}
					<li>{change}</li>
				{/each}
			</ul>
		{:else if notes}
			<div class="changelog-fallback text-muted">{notes}</div>
		{:else}
			<div class="changelog-status text-muted">No changelog available.</div>
		{/if}
	{/if}
	<div class="actions">
		<button class="btn btn-primary update-btn" onclick={onUpdate}>Update now</button>
		<button class="btn btn-secondary" onclick={onDismiss}>Later</button>
	</div>
</div>

<style>
	.update-toast {
		position: fixed;
		right: var(--space-6);
		bottom: var(--space-6);
		z-index: 60;
		width: 280px;
		background: var(--color-surface);
		border-radius: var(--radius-lg);
		padding: var(--space-4) var(--space-6);
	}

	.update-toast.inline {
		position: static;
		width: 100%;
		margin-top: var(--space-4);
		box-shadow: none;
	}

	.title {
		font-family: var(--font-heading);
		font-weight: var(--font-heading-weight);
		font-size: 14px;
		color: var(--color-text);
	}

	.details-btn {
		padding: 0;
		margin-top: var(--space-2);
		height: auto;
		font-size: 12px;
	}

	.changelog {
		list-style: disc;
		font-size: 12px;
		margin: var(--space-2) 0 0;
		line-height: 1.5;
		max-height: 120px;
		overflow-y: auto;
		padding-left: var(--space-4);
	}

	.changelog li {
		margin-bottom: 2px;
	}

	.changelog-status,
	.changelog-fallback {
		font-size: 12px;
		margin-top: var(--space-2);
		line-height: 1.5;
	}

	.changelog-fallback {
		max-height: 60px;
		overflow-y: auto;
		white-space: pre-wrap;
	}

	.actions {
		display: flex;
		gap: var(--space-2);
		margin-top: var(--space-4);
	}

	.update-btn {
		flex: 1;
	}

	@media (max-width: 767px) {
		.update-toast:not(.inline) {
			left: var(--space-4);
			right: var(--space-4);
			width: auto;
		}
	}
</style>
