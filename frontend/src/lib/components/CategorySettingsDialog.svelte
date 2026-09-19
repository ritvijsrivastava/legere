<script lang="ts">
	import { untrack } from 'svelte';
	import * as api from '$lib/api';
	import type { Category } from '$lib/types';
	import CategoryIconPicker from './CategoryIconPicker.svelte';

	let {
		open,
		category,
		onclose,
		onDeleted
	}: {
		open: boolean;
		category: Category;
		onclose: () => void;
		/** Fired after the category is actually deleted \u2014 the caller
		 *  navigates away, since this category's page no longer exists. */
		onDeleted: () => void;
	} = $props();

	let name = $state(untrack(() => category.name));
	let saving = $state(false);
	let deleting = $state(false);
	let savingIcon = $state(false);
	let error = $state<string | null>(null);
	let inputEl = $state<HTMLInputElement>();

	$effect(() => {
		if (open) {
			name = category.name;
			error = null;
			inputEl?.focus();
		}
	});

	function close() {
		if (saving || deleting) return;
		onclose();
	}

	async function changeIcon(icon: string) {
		savingIcon = true;
		error = null;
		try {
			// Emits `category:changed`, which the caller re-derives `category`
			// from — same no-local-patch-needed shape as `save()` below.
			await api.setCategoryIcon(category.id, icon);
		} catch (e) {
			error = api.errorMessage(e);
		} finally {
			savingIcon = false;
		}
	}

	async function save() {
		const trimmed = name.trim();
		if (!trimmed || trimmed === category.name) return;
		saving = true;
		error = null;
		try {
			await api.renameCategory(category.id, trimmed);
			// `rename_category` emits `category:changed`, which refreshes the
			// sidebar's aggregate list \u2014 the caller re-derives this dialog's
			// `category` prop from there, so no local patch is needed here.
			onclose();
		} catch (e) {
			error = api.errorMessage(e);
		} finally {
			saving = false;
		}
	}

	async function remove() {
		if (
			!confirm(`Delete the category "${category.name}"? Its articles will move to Uncategorized.`)
		) {
			return;
		}
		deleting = true;
		error = null;
		try {
			await api.deleteCategory(category.id);
			onDeleted();
		} catch (e) {
			error = api.errorMessage(e);
			deleting = false;
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
			aria-labelledby="category-settings-title"
			tabindex="-1"
			onclick={(e) => e.stopPropagation()}
			onkeydown={(e) => e.stopPropagation()}
		>
			<div class="dialog-title" id="category-settings-title">Category settings</div>
			<div class="field">
				<label for="category-name">Name</label>
				<div class="name-row">
					<CategoryIconPicker
						icon={category.icon}
						disabled={savingIcon}
						onselect={changeIcon}
					/>
					<input
						id="category-name"
						class="input"
						autocomplete="off"
						bind:this={inputEl}
						bind:value={name}
						onkeydown={(e) => {
							if (e.key === 'Enter' && !saving) save();
						}}
					/>
				</div>
			</div>
			{#if error}<div class="dialog-body dialog-body-error">{error}</div>{/if}
			<div class="dialog-actions">
				<button class="btn btn-ghost danger-action" onclick={remove} disabled={deleting || saving}>
					{deleting ? 'Deleting…' : 'Delete category'}
				</button>
				<span class="spacer"></span>
				<button class="btn btn-secondary" onclick={close} disabled={saving || deleting}>
					Cancel
				</button>
				<button
					class="btn btn-primary"
					onclick={save}
					disabled={saving || deleting || !name.trim() || name.trim() === category.name}
				>
					{saving ? 'Saving…' : 'Save'}
				</button>
			</div>
		</div>
	</div>
{/if}

<style>
	.dialog-actions {
		align-items: center;
	}
	.spacer {
		flex: 1;
	}
	.danger-action {
		color: var(--color-danger);
	}
	.name-row {
		display: flex;
		align-items: center;
		gap: 8px;
	}
	.name-row .input {
		flex: 1;
	}
</style>
