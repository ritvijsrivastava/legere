import * as api from '../api';
import type { NamedCount } from '../types';

/** Library-wide aggregates for the sidebar/nav (total, unread, and
 *  favorited counts, plus the category/tag lists) — computed in SQL
 *  (`count_all_articles`, `count_unread`, `count_favorited`,
 *  `list_categories`, `list_tags`) rather than derived from an in-memory
 *  copy of every article, since `ArticleCollection` no longer holds one
 *  (see `ArticlePageRequest`/keyset pagination). Refreshed on mount, on
 *  the backend's `articles:changed` event, and after any of this
 *  session's own mutations that could move a count (favorite/read/
 *  delete/sync/import). */
class LibraryStatsStore {
	loaded = $state(false);
	totalCount = $state(0);
	unreadCount = $state(0);
	favoritedCount = $state(0);
	categories = $state<NamedCount[]>([]);
	tags = $state<NamedCount[]>([]);
	/** Bumped synchronously by `notifyChanged` — `ArticleCollection`
	 *  depends on this to know when to re-query its own (independently
	 *  paginated) page 1, without this store needing to know anything
	 *  about that fetch itself. */
	changeVersion = $state(0);

	/** Called on the backend's `articles:changed` event (sync/import/
	 *  delete/etc. from anywhere in the app) — bumps `changeVersion`
	 *  immediately so listeners can react before the aggregate counts
	 *  below have actually finished refetching. */
	notifyChanged() {
		this.changeVersion++;
		this.refresh();
	}

	async refresh() {
		const [totalCount, unreadCount, favoritedCount, categories, tags] = await Promise.all([
			api.countAllArticles(),
			api.countUnread(),
			api.countFavorited(),
			api.listCategories(),
			api.listTags()
		]);
		this.totalCount = totalCount;
		this.unreadCount = unreadCount;
		this.favoritedCount = favoritedCount;
		this.categories = categories;
		this.tags = tags;
		this.loaded = true;
	}
}

export const libraryStatsStore = new LibraryStatsStore();
