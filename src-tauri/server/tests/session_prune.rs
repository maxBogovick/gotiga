//! Суточная уборка сессий: просроченные стираются все, а не только у того, кто
//! снова вошёл. Политика (/privacy) обещает именно это, и проверяется здесь то,
//! что видно только в базе: чужая просроченная строка уходит, живая остаётся.

use gotiga_server::db::Repository;
use sqlx::PgPool;
use uuid::Uuid;

#[sqlx::test]
async fn every_expired_session_is_swept_and_live_ones_stay(pool: PgPool) {
    sqlx::migrate!("./migrations/").run(&pool).await.unwrap();
    let mut ids = Vec::new();
    for name in ["Вошёл и не вернулся", "Сейчас здесь"] {
        let id: Uuid = sqlx::query_scalar("INSERT INTO users (display_name) VALUES ($1) RETURNING id")
            .bind(name)
            .fetch_one(&pool)
            .await
            .unwrap();
        ids.push(id);
    }
    // Первый не заходил после истечения сессии (с адресом и браузером), у
    // второго одна просрочена и одна жива.
    for (user, token, interval) in [
        (ids[0], "old-1", "- INTERVAL '40 days'"),
        (ids[1], "old-2", "- INTERVAL '1 day'"),
        (ids[1], "live-1", "+ INTERVAL '10 days'"),
    ] {
        sqlx::query(&format!(
            "INSERT INTO user_sessions (user_id, token, expires_at) VALUES ($1, $2, NOW() {interval})"
        ))
        .bind(user)
        .bind(token)
        .execute(&pool)
        .await
        .unwrap();
    }

    let swept = Repository::new(pool.clone()).prune_all_expired_sessions().await.unwrap();
    assert_eq!(swept, 2, "просроченные сессии остались");

    let left: Vec<String> = sqlx::query_scalar("SELECT token FROM user_sessions ORDER BY token")
        .fetch_all(&pool)
        .await
        .unwrap();
    assert_eq!(left, vec!["live-1".to_string()], "уборка тронула живую сессию");
}
