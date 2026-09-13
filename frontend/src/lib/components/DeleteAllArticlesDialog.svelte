<script lang="ts">
	import { uiStore } from '$lib/stores/ui.svelte';
	import * as api from '$lib/api';

	const CONFIRM_WORD = 'DELETE';

	let confirmValue = $state('');
	let deleting = $state(false);
	let error = $state<string | null>(null);
	let inputEl = $state<HTMLInputElement>();

	$effect(() => {
		if (uiStore.deleteAllArticlesDialogOpen) inputEl?.focus();
	});

	function close() {
		if (deleting) return;
		uiStore.closeDeleteAllArticlesDialog();
		confirmValue = '';
		error = null;
	}

	async function confirmDelete() {
		if (confirmValue !== CONFIRM_WORD || deleting) return;
		deleting = true;
		error = null;
		try {
			await api.deleteAllArticles();
			close();
		} catch (e) {
			error = api.errorMessage(e);
		} finally {
			deleting = false;
		}
	}
</script>

<svelte:window
	onkeydown={(e) => {
		if (uiStore.deleteAllArticlesDialogOpen && e.key === 'Escape') close();
	}}
/>

{#if uiStore.deleteAllArticlesDialogOpen}
	<div class="dialog-backdrop" role="presentation" onclick={close}>
		<div
			class="dialog"
			role="dialog"
			aria-modal="true"
			aria-labelledby="delete-all-title"
			tabindex="-1"
			onclick={(e) => e.stopPropagation()}
			onkeydown={(e) => e.stopPropagation()}
		>
			<div class="dialog-title" id="delete-all-title">Delete all articles</div>
			<div class="dialog-body">
				This permanently deletes every saved article and its files. Sources are kept, but
				everything captured from them is gone.
			</div>
			<div class="dialog-body dialog-body-warning">
				This action is destructive and cannot be undone.
			</div>
			<div class="field">
				<label for="delete-all-confirm">Type {CONFIRM_WORD} to confirm</label>
				<input
					id="delete-all-confirm"
					class="input"
					type="text"
					autocomplete="off"
					spellcheck="false"
					bind:this={inputEl}
					bind:value={confirmValue}
					onkeydown={(e) => {
						if (e.key === 'Enter' && !deleting && confirmValue === CONFIRM_WORD) confirmDelete();
					}}
				/>
			</div>
			{#if error}
				<div class="dialog-body dialog-body-error">{error}</div>
			{/if}
			<div class="dialog-actions">
				<button class="btn btn-secondary" onclick={close} disabled={deleting}>Cancel</button>
				<button
					class="btn btn-danger"
					onclick={confirmDelete}
					disabled={deleting || confirmValue !== CONFIRM_WORD}
				>
					{deleting ? 'Deleting\u2026' : 'Delete all articles'}
				</button>
			</div>
		</div>
	</div>
{/if}

<style>
	.dialog-body-warning {
		color: var(--color-danger);
		font-weight: 600;
	}
</style>
