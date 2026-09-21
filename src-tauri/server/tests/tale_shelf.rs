//! Postgres-backed checks for the shelf of tall tales.
//!
//! Three statements carry the room: the shelf listing, the hand reordering,
//! and the fork in `gazette_leaf_neighbors` that keeps a tale walking along
//! its own shelf instead of wandering off into the gazette. Requires
//! `DATABASE_URL` and CREATE DATABASE.

use gotiga_server::db::Repository;
use sqlx::PgPool;
use uuid::Uuid;

/// A live leaf. `day` fixes the publication date so ordering is deterministic;
/// a bigger `day` is more recent.
async fn seed_leaf(pool: &PgPool, slug: &str, kind: &str, day: i32) -> Uuid {
    sqlx::query_scalar(
        "INSERT INTO gazette_leaves (slug, kind, status, title_en, title_ru, published_at)
         VALUES ($1, $2, 'published', $1, $1, TIMESTAMPTZ '2026-01-01 00:00Z' + ($3 || ' days')::interval)
         RETURNING id",
    )
    .bind(slug)
    .bind(kind)
    .bind(day.to_string())
    .fetch_one(pool)
    .await
    .expect("insert leaf")
}

async fn shelf_slugs(repo: &Repository) -> Vec<String> {
    repo.list_tales_public(500)
        .await
        .expect("list shelf")
        .into_iter()
        .map(|l| l.slug)
        .collect()
}

#[sqlx::test]
async fn shelf_lists_only_tales_and_obeys_the_hand(pool: PgPool) {
    let repo = Repository::new(pool.clone());

    let older = seed_leaf(&pool, "tale-older", "tale", 1).await;
    let newer = seed_leaf(&pool, "tale-newer", "tale", 2).await;
    let middle = seed_leaf(&pool, "tale-middle", "tale", 3).await;
    seed_leaf(&pool, "an-arrival", "arrival", 9).await;
    seed_leaf(&pool, "a-showing", "showing", 9).await;

    // Unarranged: newest first, and nothing but tales.
    assert_eq!(
        shelf_slugs(&repo).await,
        vec!["tale-middle", "tale-newer", "tale-older"],
    );

    // Arranged by hand: the dates stop mattering entirely.
    repo.set_tale_shelf_order(&[older, middle, newer])
        .await
        .expect("reorder");
    assert_eq!(
        shelf_slugs(&repo).await,
        vec!["tale-older", "tale-middle", "tale-newer"],
    );

    // A newcomer has no place yet, so it waits at the end however recent it is.
    seed_leaf(&pool, "tale-fresh", "tale", 99).await;
    assert_eq!(
        shelf_slugs(&repo).await,
        vec!["tale-older", "tale-middle", "tale-newer", "tale-fresh"],
    );
}

#[sqlx::test]
async fn reordering_never_reaches_past_the_shelf(pool: PgPool) {
    let repo = Repository::new(pool.clone());

    let tale = seed_leaf(&pool, "tale-one", "tale", 1).await;
    let arrival = seed_leaf(&pool, "an-arrival", "arrival", 1).await;

    // A stale tab may name a leaf that is not a tale. It is skipped, not obeyed.
    let touched = repo
        .set_tale_shelf_order(&[arrival, tale])
        .await
        .expect("reorder");
    assert_eq!(touched, 1, "only the tale should be renumbered");

    let arrival_place: Option<i32> =
        sqlx::query_scalar("SELECT shelf_order FROM gazette_leaves WHERE id = $1")
            .bind(arrival)
            .fetch_one(&pool)
            .await
            .expect("read arrival");
    assert_eq!(
        arrival_place, None,
        "the gazette keeps no place on the shelf"
    );

    // An empty shelf is a no-op, not an error and not a wiped column.
    assert_eq!(repo.set_tale_shelf_order(&[]).await.expect("empty"), 0);
    assert_eq!(shelf_slugs(&repo).await, vec!["tale-one"]);
}

#[sqlx::test]
async fn a_tale_walks_its_own_shelf(pool: PgPool) {
    let repo = Repository::new(pool.clone());

    let first = seed_leaf(&pool, "tale-first", "tale", 1).await;
    let second = seed_leaf(&pool, "tale-second", "tale", 3).await;
    let third = seed_leaf(&pool, "tale-third", "tale", 5).await;
    // Laid out between the tales in time — it must never turn up as a neighbour
    // on the shelf. Every date here is distinct: the gazette breaks a tie on a
    // random uuid, which would make this test flip a coin.
    seed_leaf(&pool, "an-arrival", "arrival", 4).await;

    repo.set_tale_shelf_order(&[first, second, third])
        .await
        .expect("reorder");

    let (prev, next) = repo
        .gazette_leaf_neighbors("tale-second", "tale")
        .await
        .expect("neighbours");
    assert_eq!(prev.map(|n| n.slug), Some("tale-first".to_string()));
    assert_eq!(next.map(|n| n.slug), Some("tale-third".to_string()));

    // Both ends of the shelf are open air.
    let (prev, next) = repo
        .gazette_leaf_neighbors("tale-first", "tale")
        .await
        .expect("neighbours");
    assert!(prev.is_none(), "nothing stands left of the first tale");
    assert_eq!(next.map(|n| n.slug), Some("tale-second".to_string()));

    // The gazette still walks in time, and still sees every kind of leaf.
    let (prev, next) = repo
        .gazette_leaf_neighbors("an-arrival", "arrival")
        .await
        .expect("neighbours");
    assert_eq!(prev.map(|n| n.slug), Some("tale-third".to_string()));
    assert!(next.is_some(), "the gazette keeps reading past the tales");
}
