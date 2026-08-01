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

	async markAsRead(id: string) {
		await api.markAsRead(id);
		const article = this.items.find((a) => a.id === id);
		if (article) article.reading_state = 'read';
	}

	get unreadCount(): number {
		return this.items.filter((a) => a.reading_state === 'unread').length;
	}
}

export const articlesStore = new ArticlesStore();
