//! Пределы «столько-то в час» лежат в базе: перезапуск сервера их не обнуляет,
//! две копии на одной базе считают сообща, параллельные запросы не проскакивают.

use gotiga_server::db::Repository;
use sqlx::PgPool;

#[sqlx::test]
async fn the_limit_holds_and_is_per_key(pool: PgPool) {
    sqlx::migrate!("./migrations/").run(&pool).await.unwrap();
    let repo = Repository::new(pool.clone());

    for _ in 0..3 {
        assert!(repo.rate_limit_take("login|1.1.1.1", 3, 900).await.unwrap());
    }
    assert!(!repo.rate_limit_take("login|1.1.1.1", 3, 900).await.unwrap(), "четвёртое прошло");
    assert!(repo.rate_limit_take("login|2.2.2.2", 3, 900).await.unwrap(), "чужой ключ задет");

    // Отказанное обращение не записывается: строк не больше предела.
    let rows: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM rate_limit_hits WHERE key = 'login|1.1.1.1'")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(rows, 3);
}

#[sqlx::test]
async fn the_limit_survives_a_new_repository_and_expires_with_the_window(pool: PgPool) {
    sqlx::migrate!("./migrations/").run(&pool).await.unwrap();
    for _ in 0..2 {
        assert!(Repository::new(pool.clone()).rate_limit_take("k", 2, 3600).await.unwrap());
    }
    // «Перезапуск»: новый объект над той же базой помнит.
    assert!(!Repository::new(pool.clone()).rate_limit_take("k", 2, 3600).await.unwrap());

    // Обращения старше окна не считаются, а уборка их стирает.
    sqlx::query("UPDATE rate_limit_hits SET hit_at = NOW() - INTERVAL '2 hours'")
        .execute(&pool)
        .await
        .unwrap();
    let repo = Repository::new(pool.clone());
    assert!(repo.rate_limit_take("k", 2, 3600).await.unwrap(), "истёкшие всё ещё считаются");
    assert_eq!(repo.prune_rate_limit_hits(3600).await.unwrap(), 2);
}

#[sqlx::test]
async fn concurrent_requests_cannot_jump_the_limit(pool: PgPool) {
    sqlx::migrate!("./migrations/").run(&pool).await.unwrap();
    let mut tasks = Vec::new();
    for _ in 0..20 {
        let repo = Repository::new(pool.clone());
        tasks.push(tokio::spawn(async move { repo.rate_limit_take("burst", 5, 3600).await.unwrap() }));
    }
    let mut taken = 0;
    for t in tasks {
        if t.await.unwrap() {
            taken += 1;
        }
    }
    assert_eq!(taken, 5, "параллельные запросы прошли мимо предела");
}
