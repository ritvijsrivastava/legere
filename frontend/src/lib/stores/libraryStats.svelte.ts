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
		// Uncategorized is always first, ahead of the real (alphabetically
		// sorted, see `fetch_categories`) categories — it's the default
		// destination, not just another folder, and should stay easy to find
		// even at 0 articles as long as there's at least one real category
		// to move something into it from.
		this.categories =
			categories.length > 0 || uncategorizedCount > 0
				? [
					{ id: '__uncategorized__', name: 'Uncategorized', article_count: uncategorizedCount },
					...categories
				]
				: categories;
		this.tags = tags;
		this.loaded = true;
	}
}

export const libraryStatsStore = new LibraryStatsStore();
