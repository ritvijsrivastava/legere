/** Sidebar-driven filters (Categories/Tags in Shell.svelte), read by
 *  `ArticleCollection` and applied server-side (see `ArticlePageRequest`)
 *  alongside whatever view (Library/Favorites) and search term it's
 *  already scoped to. */
class LibraryFiltersStore {
	sourceName = $state<string | null>(null);
	tags = $state<string[]>([]);

	toggleSource(name: string) {
		this.sourceName = this.sourceName === name ? null : name;
	}

	toggleTag(tag: string) {
		this.tags = this.tags.includes(tag) ? this.tags.filter((t) => t !== tag) : [...this.tags, tag];
	}

	clear() {
		this.sourceName = null;
		this.tags = [];
	}
}

export const libraryFiltersStore = new LibraryFiltersStore();
