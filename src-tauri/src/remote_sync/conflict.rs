//! Deterministic conflict resolution for synced rows sharing a
//! `conflict_key` (see ARCHITECTURE.md's Sync section): two devices can
//! independently create "the same" article (same `link`), category (same
//! case-insensitive name), or source (same `feed_url`) before either has
//! synced. Every device must resolve the race to the *same* winner
//! regardless of which device resolves it first or what order syncs
//! happen in — that's what makes this automerge-style rather than plain
//! "whoever pushes last wins."
//!
//! This module is intentionally pure/data-only — no I/O, no DB, no
//! network — so the actual merge property (order-independence) is cheap
//! to verify with unit tests, independent of the sync engine that calls
//! it.

use std::cmp::Ordering;

/// The minimum a caller needs to know about one side of a same-key race.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Candidate {
    pub id: String,
    /// The entity's own `created_at`/`fetched_at` — whichever column
    /// means "when this row first came into existence," not
    /// `updated_at`, which keeps moving as mutable fields change and
    /// would make the winner unstable over time.
    pub created_at: String,
}

/// Picks the deterministic winner between two rows sharing a
/// `conflict_key`. Smaller `(created_at, id)` wins — earlier creation,
/// tie-broken by id string ordering so a same-timestamp race still
/// resolves identically on every device. Returns `(winner, loser)`.
pub fn pick_winner<'a>(a: &'a Candidate, b: &'a Candidate) -> (&'a Candidate, &'a Candidate) {
    let key = |c: &'a Candidate| (c.created_at.as_str(), c.id.as_str());
    match key(a).cmp(&key(b)) {
        Ordering::Less | Ordering::Equal => (a, b),
        Ordering::Greater => (b, a),
    }
}

/// Mutable per-article state that must be folded from a collision loser
/// into the winner before the loser is tombstoned — otherwise a harmless
/// capture race silently discards real user actions (tags, read
/// progress, favorites) taken on the losing copy before the race was
/// noticed. Categories/sources carry no per-row mutable state worth
/// merging this way (a source's `article_count` is a cache the normal
/// article-repoint step already recomputes, matching schema `V16`'s
/// dedupe behavior).
#[derive(Debug, Clone, PartialEq)]
pub struct ArticleMutableState {
    pub tags: Vec<String>,
    pub reading_progress: f64,
    pub favorited: bool,
}

/// Unions tags, takes the further-along reading progress, and ORs the
/// favorited flag — the merge is symmetric (order of `winner`/`loser`
/// doesn't change the result), matching `pick_winner`'s own
/// order-independence.
pub fn merge_article_state(
    winner: &ArticleMutableState,
    loser: &ArticleMutableState,
) -> ArticleMutableState {
    let mut tags = winner.tags.clone();
    for tag in &loser.tags {
        if !tags.contains(tag) {
            tags.push(tag.clone());
        }
    }
    tags.sort();

    ArticleMutableState {
        tags,
        reading_progress: winner.reading_progress.max(loser.reading_progress),
        favorited: winner.favorited || loser.favorited,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn earlier_created_at_wins_regardless_of_argument_order() {
        let earlier = Candidate {
            id: "z-later-id".into(),
            created_at: "2026-01-01T00:00:00Z".into(),
        };
        let later = Candidate {
            id: "a-earlier-id".into(),
            created_at: "2026-01-02T00:00:00Z".into(),
        };

        let (winner, loser) = pick_winner(&earlier, &later);
        assert_eq!(winner.id, "z-later-id");
        assert_eq!(loser.id, "a-earlier-id");

        // Swapped argument order must produce the identical result —
        // this is the property that makes merge order not matter.
        let (winner2, loser2) = pick_winner(&later, &earlier);
        assert_eq!(winner2.id, winner.id);
        assert_eq!(loser2.id, loser.id);
    }

    #[test]
    fn same_created_at_ties_break_on_id_string_order() {
        let a = Candidate {
            id: "aaa".into(),
            created_at: "2026-01-01T00:00:00Z".into(),
        };
        let b = Candidate {
            id: "bbb".into(),
            created_at: "2026-01-01T00:00:00Z".into(),
        };

        let (winner, _) = pick_winner(&a, &b);
        assert_eq!(winner.id, "aaa");
        let (winner2, _) = pick_winner(&b, &a);
        assert_eq!(winner2.id, "aaa");
    }

    #[test]
    fn merge_article_state_unions_tags_dedups_and_sorts() {
        let winner = ArticleMutableState {
            tags: vec!["rust".into(), "webdev".into()],
            reading_progress: 0.2,
            favorited: false,
        };
        let loser = ArticleMutableState {
            tags: vec!["webdev".into(), "async".into()],
            reading_progress: 0.7,
            favorited: true,
        };

        let merged = merge_article_state(&winner, &loser);
        assert_eq!(merged.tags, vec!["async", "rust", "webdev"]);
        assert_eq!(
            merged.reading_progress, 0.7,
            "further-along progress must survive"
        );
        assert!(merged.favorited, "favorited on either side must survive");
    }

    #[test]
    fn merge_article_state_is_order_independent() {
        let a = ArticleMutableState {
            tags: vec!["a".into()],
            reading_progress: 0.1,
            favorited: false,
        };
        let b = ArticleMutableState {
            tags: vec!["b".into()],
            reading_progress: 0.9,
            favorited: true,
        };

        assert_eq!(merge_article_state(&a, &b), merge_article_state(&b, &a));
    }
}
