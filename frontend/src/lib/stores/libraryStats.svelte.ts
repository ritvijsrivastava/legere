import * as api from '../api';
import type { Category, NamedCount } from '../types';

/** Library-wide aggregates for the sidebar/nav (total, unread, and
 *  favorited counts, plus real folder categories and tags) — computed in
 *  SQL rather than derived from an in-memory copy of every article, since
 *  `ArticleCollection` only holds the current paginated result. */
class LibraryStatsStore {
	loaded = $state(false);
	totalCount = $state(0);
	unreadCount = $state(0);
	favoritedCount = $state(0);
	categories = $state<Category[]>([]);
	tags = $state<NamedCount[]>([]);
	/** Bumped synchronously by `notifyChanged` so ArticleCollection can
	 *  refetch its current page after a category/article mutation. */
	changeVersion = $state(0);

	notifyChanged() {
		this.changeVersion++;
		this.refresh();
	}

	async refresh() {
		const [totalCount, unreadCount, favoritedCount, categories, uncategorizedCount, tags] =
			await Promise.all([
				api.countAllArticles(),
				api.countUnread(),
				api.countFavorited(),
				api.getCategories(),
				api.countUncategorized(),
				api.listTags()
			]);
		this.totalCount = totalCount;
		this.unreadCount = unreadCount;
		this.favoritedCount = favoritedCount;
		this.categories =
			uncategorizedCount > 0
				? [
					...categories,
					{ id: '__uncategorized__', name: 'Uncategorized', article_count: uncategorizedCount }
				]
				: categories;
		this.tags = tags;
		this.loaded = true;
	}
}

export const libraryStatsStore = new LibraryStatsStore();
