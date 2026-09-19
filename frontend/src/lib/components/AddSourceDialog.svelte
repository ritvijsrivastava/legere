<script lang="ts">
	import { uiStore } from '$lib/stores/ui.svelte';
	import { captureJobsStore } from '$lib/stores/captureJobs.svelte';
	import * as api from '$lib/api';

	let newSourceValue = $state('');
	let submitting = $state(false);
	let error = $state<string | null>(null);
	let inputEl = $state<HTMLInputElement>();

	$effect(() => {
		if (uiStore.addSourceOpen) inputEl?.focus();
	});

	function reset() {
		newSourceValue = '';
		error = null;
	}

	function close() {
		uiStore.closeAddSource();
		reset();
	}

	// The actual fetch/extract/localize pipeline (which is what can take a
	// while \u2014 an image-heavy article, or a slow site) runs in the
	// background: this only waits long enough to queue the job, then
	// closes immediately. A freshly captured direct-link article always
	// starts Uncategorized (new articles are inserted with no
	// `category_id` \u2014 see `db::queries`) rather than prompting for a
	// category up front; moving it somewhere else is one action away from
	// its card or the reader (see `MoveToCategoryDialog`). Progress,
	// success, and any failure (with a retry) surface via
	// `captureJobsStore`/`CaptureJobsPanel`, not this dialog.
	async function submit() {
		const value = newSourceValue.trim();
		if (!value) return;
		submitting = true;
		error = null;
		try {
			await api.addSourceBackground(value);
			await captureJobsStore.refresh();
			close();
		} catch (e) {
			error = api.errorMessage(e);
		} finally {
			submitting = false;
		}
	}
</script>

<svelte:window
	onkeydown={(e) => {
		if (uiStore.addSourceOpen && e.key === 'Escape') close();
	}}
/>

{#if uiStore.addSourceOpen}
	<div class="dialog-backdrop" role="presentation" onclick={close}>
		<div
			class="dialog"
			role="dialog"
			aria-modal="true"
			aria-labelledby="add-source-title"
			tabindex="-1"
			onclick={(e) => e.stopPropagation()}
			onkeydown={(e) => e.stopPropagation()}
		>
			<div class="dialog-title" id="add-source-title">Add a source</div>

			<div class="field">
				<label for="new-source-value">Feed or article URL</label>
				<input
					id="new-source-value"
					class="input"
					type="text"
					placeholder="https://example.com/feed-or-article"
					autocomplete="off"
					spellcheck="false"
					bind:this={inputEl}
					bind:value={newSourceValue}
					onkeydown={(e) => {
						if (e.key === 'Enter' && !submitting && newSourceValue.trim()) submit();
					}}
				/>
			</div>
			<div class="dialog-body">
				Legere figures out which kind of link this is. A feed is checked periodically for new
				entries; anything else is captured once as a standalone article, filed under
				Uncategorized until you move it.
			</div>

			{#if error}
				<div class="dialog-body dialog-body-error">{error}</div>
			{/if}
			<div class="dialog-actions">
				<button class="btn btn-secondary" onclick={close}>Cancel</button>
				<button class="btn btn-primary" onclick={submit} disabled={submitting || !newSourceValue.trim()}>
					{submitting ? 'Adding…' : 'Add source'}
				</button>
			</div>
		</div>
	</div>
{/if}
