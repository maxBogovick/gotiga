//! Postgres-backed checks for `Repository::set_figurine_like`.
//!
//! Two tests share one migrated database each: the happy path, then the
//! cases that must fail closed. Requires `DATABASE_URL` and CREATE DATABASE.

use gotiga_server::db::Repository;
use sqlx::PgPool;
use uuid::Uuid;

async fn seed_figurine(pool: &PgPool) -> Uuid {
    sqlx::query_scalar("INSERT INTO figurines (name) VALUES ('like-test') RETURNING id")
        .fetch_one(pool)
        .await
        .expect("insert figurine")
}

async fn seed_user(pool: &PgPool, tag: &str) -> Uuid {
    sqlx::query_scalar(
        "INSERT INTO users (email, display_name, visual_password_hash)
         VALUES ($1, $2, 'x') RETURNING id",
    )
    .bind(format!("like-{tag}-{}@example.test", Uuid::new_v4()))
    .bind(tag)
    .fetch_one(pool)
    .await
    .expect("insert user")
}

async fn like_count(pool: &PgPool, figurine_id: Uuid) -> i64 {
    sqlx::query_scalar("SELECT COUNT(*)::bigint FROM figurine_likes WHERE figurine_id = $1")
        .bind(figurine_id)
        .fetch_one(pool)
        .await
        .expect("count likes")
}

async fn like_owners(pool: &PgPool, figurine_id: Uuid) -> Vec<(String, Option<Uuid>)> {
    sqlx::query_as(
        "SELECT visitor_token, user_id FROM figurine_likes
         WHERE figurine_id = $1
         ORDER BY created_at, visitor_token",
    )
    .bind(figurine_id)
    .fetch_all(pool)
    .await
    .expect("list like rows")
}

async fn wishlist(pool: &PgPool, user_id: Uuid) -> Vec<String> {
    sqlx::query_scalar("SELECT wishlist FROM users WHERE id = $1")
        .bind(user_id)
        .fetch_one(pool)
        .await
        .expect("read wishlist")
}

#[sqlx::test]
async fn like_happy_paths(pool: PgPool) {
    let repo = Repository::new(pool.clone());

    // Guest like, then the same token after login — one row, wishlist updated.
    let fig = seed_figurine(&pool).await;
    let user = seed_user(&pool, "login").await;
    let token = "token-shared-bbbbbbbbbbbbbbbb";
    let (liked, count) = repo
        .set_figurine_like(fig, token, None, true)
        .await
        .unwrap();
    assert!(liked && count == 1);
    let (liked, count) = repo
        .set_figurine_like(fig, token, Some(user), true)
        .await
        .unwrap();
    assert!(liked);
    assert_eq!(count, 1, "login must absorb the guest row");
    assert_eq!(
        like_owners(&pool, fig).await,
        vec![(token.to_string(), Some(user))]
    );
    assert_eq!(wishlist(&pool, user).await, vec![fig.to_string()]);

    // Guest like on the bound token must not strip user_id.
    repo.set_figurine_like(fig, token, None, true)
        .await
        .unwrap();
    assert_eq!(
        like_owners(&pool, fig).await,
        vec![(token.to_string(), Some(user))]
    );

    // Second device of the same account replaces the first row, no double count.
    repo.set_figurine_like(fig, "token-laptop-fffffffffffff", Some(user), true)
        .await
        .unwrap();
    assert_eq!(like_count(&pool, fig).await, 1);
    assert_eq!(
        like_owners(&pool, fig).await,
        vec![("token-laptop-fffffffffffff".into(), Some(user))]
    );

    // Account row + a guest row on another token, then login on that token:
    // unique (figurine, visitor_token) must not 500.
    let fig2 = seed_figurine(&pool).await;
    let user2 = seed_user(&pool, "device").await;
    repo.set_figurine_like(fig2, "token-device-one-ccccccccc", Some(user2), true)
        .await
        .unwrap();
    repo.set_figurine_like(fig2, "token-device-two-ddddddddd", None, true)
        .await
        .unwrap();
    let (liked, count) = repo
        .set_figurine_like(fig2, "token-device-two-ddddddddd", Some(user2), true)
        .await
        .expect("login on the guest device");
    assert!(liked);
    assert_eq!(count, 1);
    assert_eq!(
        like_owners(&pool, fig2).await,
        vec![("token-device-two-ddddddddd".into(), Some(user2))]
    );

    // Unlike after logout still clears the account wishlist.
    let (liked, count) = repo
        .set_figurine_like(fig2, "token-device-two-ddddddddd", None, false)
        .await
        .unwrap();
    assert!(!liked && count == 0);
    assert!(like_owners(&pool, fig2).await.is_empty());
    assert!(wishlist(&pool, user2).await.is_empty());

    // Two concurrent account likes collapse to one row.
    let fig3 = seed_figurine(&pool).await;
    let user3 = seed_user(&pool, "race").await;
    let left = repo.clone();
    let right = repo.clone();
    let (a, b) = tokio::join!(
        left.set_figurine_like(fig3, "token-race-left-kkkkkkkk", Some(user3), true),
        right.set_figurine_like(fig3, "token-race-right-lllllll", Some(user3), true),
    );
    assert!(a.is_ok(), "left like failed: {a:?}");
    assert!(b.is_ok(), "right like failed: {b:?}");
    assert_eq!(like_count(&pool, fig3).await, 1);
    assert_eq!(wishlist(&pool, user3).await, vec![fig3.to_string()]);
}

#[sqlx::test]
async fn like_failures_stay_closed(pool: PgPool) {
    let repo = Repository::new(pool.clone());

    // Failed INSERT (unknown figurine) must not still append wishlist.
    let user = seed_user(&pool, "fail").await;
    let missing = Uuid::new_v4();
    let err = repo
        .set_figurine_like(missing, "token-guest-aaaaaaaaaaaaaaaa", Some(user), true)
        .await;
    assert!(err.is_err(), "FK failure must surface: {err:?}");
    assert!(wishlist(&pool, user).await.is_empty());
    assert_eq!(like_count(&pool, missing).await, 0);

    // A stranger's unlike must not remove someone else's like or wishlist.
    let fig = seed_figurine(&pool).await;
    let owner = seed_user(&pool, "keep").await;
    repo.set_figurine_like(fig, "token-owner-hhhhhhhhhhhhhh", Some(owner), true)
        .await
        .unwrap();
    repo.set_figurine_like(fig, "token-stranger-iiiiiiiiiii", None, false)
        .await
        .unwrap();
    assert_eq!(like_count(&pool, fig).await, 1);
    assert_eq!(wishlist(&pool, owner).await, vec![fig.to_string()]);
}
