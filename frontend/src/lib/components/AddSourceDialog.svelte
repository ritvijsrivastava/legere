<script lang="ts">
	import { goto } from '$app/navigation';
	import { uiStore } from '$lib/stores/ui.svelte';
	import { sourcesStore } from '$lib/stores/sources.svelte';
	import * as api from '$lib/api';
	import SegmentedControl from './SegmentedControl.svelte';
	import type { SourceType } from '$lib/types';

	let newSourceType = $state<SourceType>('rss');
	let newSourceValue = $state('');
	let submitting = $state(false);
	let error = $state<string | null>(null);

	const typeLabelMap: Record<SourceType, string> = {
		rss: 'RSS feed URL',
		mail: 'Forwarding address',
		direct: 'Article URL'
	};
	const placeholderMap: Record<SourceType, string> = {
		rss: 'https://example.com/feed.xml',
		mail: 'you@company.com',
		direct: 'https://example.com/article'
	};
	const helpMap: Record<SourceType, string> = {
		rss: 'Legere checks this feed periodically and stores new entries for offline reading.',
		mail: 'Forward emails to this address and Legere will scrape and store them as articles.',
		direct: 'Paste a link and Legere scrapes the page once and saves a clean copy.'
	};

	function close() {
		uiStore.closeAddSource();
		newSourceValue = '';
		newSourceType = 'rss';
		error = null;
	}

	async function submit() {
		if (!newSourceValue.trim()) return;
		submitting = true;
		error = null;
		try {
			if (newSourceType === 'rss') {
				await sourcesStore.add('rss', newSourceValue.trim());
				close();
			} else if (newSourceType === 'direct') {
				const article = await api.addDirectLinkArticle(newSourceValue.trim());
				close();
				await goto(`/reader/${article.id}`);
			}
		} catch (e) {
			error = e instanceof Error ? e.message : String(e);
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
			<SegmentedControl
				name="newtype"
				bind:value={newSourceType as unknown as string}
				options={[
					{ value: 'rss', label: 'RSS' },
					{ value: 'mail', label: 'Mail' },
					{ value: 'direct', label: 'Direct link' }
				]}
			/>
			<div class="field">
				<label for="new-source-value">{typeLabelMap[newSourceType]}</label>
				<input
					id="new-source-value"
					class="input"
					type="text"
					placeholder={placeholderMap[newSourceType]}
					bind:value={newSourceValue}
				/>
			</div>
			<div class="dialog-body">
				{helpMap[newSourceType]}
				{#if newSourceType === 'mail'}
					<br /><strong>Mail sources are coming in a future release.</strong>
				{/if}
			</div>
			{#if error}
				<div class="dialog-body">{error}</div>
			{/if}
			<div class="dialog-actions">
				<button class="btn btn-secondary" onclick={close}>Cancel</button>
				<button
					class="btn btn-primary"
					onclick={submit}
					disabled={newSourceType === 'mail' || submitting || !newSourceValue.trim()}
				>
					Add source
				</button>
			</div>
		</div>
	</div>
{/if}
