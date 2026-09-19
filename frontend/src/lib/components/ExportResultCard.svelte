<script lang="ts">
	import type { ExportResult } from '$lib/types';
	import * as api from '$lib/api';
	import { formatRelativeTime } from '$lib/format';

	let { result, defaultSaveName }: { result: ExportResult; defaultSaveName: string } = $props();

	let revealing = $state(false);
	let saving = $state(false);
	let error = $state<string | null>(null);

	async function reveal() {
		error = null;
		revealing = true;
		try {
			await api.revealItemInDir(result.path);
		} catch (e) {
			error = api.errorMessage(e);
		} finally {
			revealing = false;
		}
	}

	async function saveCopy() {
		error = null;
		saving = true;
		try {
			const dest = await api.saveCsvFile({
				defaultPath: defaultSaveName,
				filters: [{ name: 'CSV', extensions: ['csv'] }]
			});
			if (!dest) return;
			const contents = await api.readTextFile(result.path);
			await api.writeTextFile(dest, contents);
		} catch (e) {
			error = api.errorMessage(e);
		} finally {
			saving = false;
		}
	}
</script>

<div class="export-result">
	<p class="filename">{result.filename}</p>
	<p class="text-muted meta">
		{result.row_count} row{result.row_count === 1 ? '' : 's'} · exported {formatRelativeTime(
			result.exported_at
		)}
	</p>
	<p class="text-muted path">{result.path}</p>
	<div class="actions">
		<button class="btn btn-ghost action-btn" disabled={revealing} onclick={reveal}>
			{revealing ? 'Opening…' : 'Reveal in folder'}
		</button>
		<button class="btn btn-ghost action-btn" disabled={saving} onclick={saveCopy}>
			{saving ? 'Saving…' : 'Save a copy…'}
		</button>
	</div>
	{#if error}
		<p class="error-text">{error}</p>
	{/if}
</div>

<style>
	.export-result {
		margin-top: 12px;
		padding-top: 12px;
		border-top: 1px solid var(--color-divider);
	}
	.filename {
		font-family: var(--font-heading);
		font-weight: 600;
		font-size: 13px;
		margin: 0;
		word-break: break-all;
	}
	.meta {
		font-size: 12px;
		margin: 4px 0 0;
	}
	.path {
		font-size: 11px;
		margin: 2px 0 0;
		word-break: break-all;
	}
	.actions {
		display: flex;
		flex-wrap: wrap;
		gap: 4px 16px;
		margin-top: 8px;
	}
	.action-btn {
		padding: 4px 0;
		font-size: 12px;
	}
	.error-text {
		font-size: 12px;
		color: var(--color-danger);
		margin: 6px 0 0;
	}
</style>
