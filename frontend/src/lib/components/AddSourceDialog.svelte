<script lang="ts">
	import { goto } from '$app/navigation';
	import { uiStore } from '$lib/stores/ui.svelte';
	import { sourcesStore } from '$lib/stores/sources.svelte';
	import * as api from '$lib/api';

	let newSourceValue = $state('');
	let submitting = $state(false);
	let error = $state<string | null>(null);
	let inputEl = $state<HTMLInputElement>();

	$effect(() => {
		if (uiStore.addSourceOpen) inputEl?.focus();
	});

	function close() {
		uiStore.closeAddSource();
		newSourceValue = '';
		error = null;
	}

	async function submit() {
		const value = newSourceValue.trim();
		if (!value) return;
		submitting = true;
		error = null;
		try {
			const result = await sourcesStore.addAuto(value);
			close();
			if (result.kind === 'direct') {
				await goto(`/reader/${result.value.id}`);
			}
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
				entries; anything else is captured once as a standalone article.
			</div>
			{#if error}
				<div class="dialog-body dialog-body-error">{error}</div>
			{/if}
			<div class="dialog-actions">
				<button class="btn btn-secondary" onclick={close}>Cancel</button>
				<button class="btn btn-primary" onclick={submit} disabled={submitting || !newSourceValue.trim()}>
					{submitting ? 'Adding\u2026' : 'Add source'}
				</button>
			</div>
		</div>
	</div>
{/if}
