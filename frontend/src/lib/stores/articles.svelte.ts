import * as api from '../api';
import type { ArticleSummary } from '../types';

class ArticlesStore {
	items = $state<ArticleSummary[]>([]);
	loading = $state(false);

	async refresh() {
		this.loading = true;
		try {
			this.items = await api.listArticles();
		} finally {
			this.loading = false;
		}
	}

	async toggleFavorite(id: string) {
		const favorited = await api.toggleFavorite(id);
		const article = this.items.find((a) => a.id === id);
		if (article) article.favorited = favorited;
	}

	async markRead(id: string) {
		await api.markRead(id);
		const article = this.items.find((a) => a.id === id);
		if (article) article.unread = false;
	}

	get unreadCount(): number {
		return this.items.filter((a) => a.unread).length;
	}
}

export const articlesStore = new ArticlesStore();
