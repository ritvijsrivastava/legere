/** Sidebar-driven filters (Categories/Tags in Shell.svelte), read by
 *  `ArticleCollection` and applied server-side (see `ArticlePageRequest`)
 *  alongside whatever view (Library/Favorites) and search term it's
 *  already scoped to. Categories are real folder ids; the reserved
 *  `__uncategorized__` value represents articles with no category. */
class LibraryFiltersStore {
	categoryId = $state<string | null>(null);
	tags = $state<string[]>([]);

	toggleCategory(id: string) {
		this.categoryId = this.categoryId === id ? null : id;
	}

	toggleTag(tag: string) {
		this.tags = this.tags.includes(tag) ? this.tags.filter((t) => t !== tag) : [...this.tags, tag];
	}

	clear() {
		this.categoryId = null;
		this.tags = [];
	}
}

export const libraryFiltersStore = new LibraryFiltersStore();
