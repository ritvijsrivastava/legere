/** Sidebar-driven filters (Categories/Tags in Shell.svelte) applied on
 *  top of whatever view (Library/Favorites) and search term a page has
 *  already narrowed down to. There's no server-side query for this —
 *  the whole library is already in memory via `articlesStore`. */
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
