use gotiga_server::{api, config, db, logs, services};
use sqlx::postgres::PgPoolOptions;
use std::net::SocketAddr;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();

    // 1. Load Config
    let config = config::Config::from_env();
    let log_store = logs::AdminLogStore::open(&config.admin_log_db_path).await?;
    let admin_log_layer = log_store.layer();

    // 2. Setup Logging
    let env_filter = tracing_subscriber::EnvFilter::new(&config.rust_log);
    if matches!(dotenvy::var("LOG_FORMAT").as_deref(), Ok("json")) {
        tracing_subscriber::registry()
            .with(env_filter)
            .with(admin_log_layer)
            .with(
                tracing_subscriber::fmt::layer()
                    .json()
                    .with_current_span(false)
                    .with_span_list(false),
            )
            .init();
    } else {
        tracing_subscriber::registry()
            .with(env_filter)
            .with(admin_log_layer)
            .with(tracing_subscriber::fmt::layer().compact())
            .init();
    }

    // NB: never log database_url — it contains DB credentials.
    tracing::info!("Starting server on {}:{}", config.host, config.port);

    // 3. Connect to DB
    // Pool is sized for a public web server: several handlers issue a handful of
    // sequential queries per request (get_figurine_details, sitemap/feed), so 5
    // connections queued behind a 30s default acquire timeout could stall under
    // even light concurrency. Tunable via DB_MAX_CONNECTIONS; timeouts are explicit
    // so a saturated pool fails fast (5s) rather than hanging, and idle/aged
    // connections are recycled.
    let db_max_connections: u32 = dotenvy::var("DB_MAX_CONNECTIONS")
        .ok()
        .and_then(|v| v.parse().ok())
        .filter(|n| *n > 0)
        .unwrap_or(20);
    let pool = PgPoolOptions::new()
        .max_connections(db_max_connections)
        .acquire_timeout(std::time::Duration::from_secs(5))
        .idle_timeout(std::time::Duration::from_secs(600))
        .max_lifetime(std::time::Duration::from_secs(1800))
        .before_acquire(|conn, _meta| {
            Box::pin(async move {
                db::clear_stale_prepared_statements(conn).await?;
                Ok(true)
            })
        })
        .connect(&config.database_url)
        .await
        .expect("Failed to connect to Postgres");

    // 4. Run Migrations (runtime path — picks up new .sql files without recompilation)
    sqlx::migrate::Migrator::new(std::path::Path::new("./migrations"))
        .await
        .expect("Failed to load migrations")
        .run(&pool)
        .await
        .expect("Failed to run migrations");

    // 5. Initialize Layers
    let repo = db::Repository::new(pool);
    let service = services::AppService::new(repo, config.clone());

    // Initialize content pool (hot load active release)
    if let Err(e) = service.initialize().await {
        tracing::warn!(
            "Failed to load active release: {}. Server will start in empty mode.",
            e
        );
    }

    // Background: rebuild image renditions that predate the current pipeline — the 900px
    // `medium` variant (new) and every `.webp` (previously encoded lossless, i.e. 6-7x
    // LARGER than its JPEG). Rebuilt from images/original, idempotent, and detached so it
    // never delays the listener.
    {
        let upload_dir = config.upload_dir.clone();
        tokio::spawn(async move {
            api::backfill_image_variants(upload_dir).await;
        });
    }

    // Снаряжение штурма: картинки стены, башни и тарана встроены в сервер и
    // кладутся на склад при запуске — так они приезжают с выкладкой, а не
    // загружаются руками (`siege.rs`). Неудача не роняет запуск: без
    // картинки карта рисуется пергаментом, а журнал говорит, почему.
    match gotiga_server::siege::lay_gear_art(std::path::Path::new(&config.upload_dir)) {
        Ok(0) => {}
        Ok(n) => tracing::info!("Laid {n} siege gear pictures into the store"),
        Err(e) => tracing::warn!("Siege gear pictures were not laid: {e}"),
    }

    // Готовые обложки книг небылиц: встроены в сервер и кладутся на склад при
    // запуске (`tale_covers.rs`), миграция только ссылается на них.
    match gotiga_server::tale_covers::lay_cover_art(std::path::Path::new(&config.upload_dir)) {
        Ok(0) => {}
        Ok(n) => tracing::info!("Laid {n} book covers into the store"),
        Err(e) => tracing::warn!("Book covers were not laid: {e}"),
    }

    // Background: give an existing main background (uploaded before the resize/
    // WebP pipeline existed) the same treatment a fresh upload gets now. Idempotent
    // and detached — see backfill_background_image's doc comment.
    {
        let upload_dir = config.upload_dir.clone();
        tokio::spawn(async move {
            api::backfill_background_image(upload_dir).await;
        });
    }

    // Гео-база: выкладка раз в месяц заменяет файл на диске, а сервер читал его
    // только при запуске. Раз в час сверяем время изменения файла.
    {
        let svc = service.clone();
        tokio::spawn(async move {
            let mut tick = tokio::time::interval(std::time::Duration::from_secs(3600));
            loop {
                tick.tick().await;
                let _ = svc
                    .track_job("geoip_reload", 3600, async { svc.reload_geoip(); Ok(()) }, |_| None)
                    .await;
            }
        });
    }

    // Background: недельный разбор быстрых уходов. Тик — час: недели «кончаются» только в
    // календаре, события у этого нет. Проход идемпотентен (разбор недели один, письмо
    // владельцу отмечается), первый тик через две минуты после запуска.
    {
        let svc = service.clone();
        tokio::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_secs(120)).await;
            let mut tick = tokio::time::interval(std::time::Duration::from_secs(3600));
            loop {
                tick.tick().await;
                let round = svc
                    .track_job("quick_exit_digest", 3600, svc.weekly_digest_round(), |(made, sent)| {
                        match (*made, *sent) {
                            (false, 0) => None,
                            (true, 0) => Some("the week's digest was made".into()),
                            (true, n) => Some(format!("the week's digest was made and sent ({n})")),
                            (false, n) => Some(format!("a digest was sent to Telegram ({n})")),
                        }
                    })
                    .await;
                match round {
                    Ok((_, n)) if n > 0 => tracing::info!("Digest of quick exits: {n} sent"),
                    Ok(_) => {}
                    Err(e) => tracing::warn!("Digest round failed: {e}"),
                }
            }
        });
    }

    // Background: вердикт сезонов студии.
    //
    // Тик — четверть часа, а не сутки: вердикт назначен на конкретную минуту
    // ночи с субботы на воскресенье, и суточный тик подвёл бы его когда
    // придётся. Задача сама идемпотентна (`judge_studio_season` первым делом
    // ставит `judged` одним запросом с условием), поэтому частый тик безопасен,
    // а простой сервера на выходных не теряет ничью неделю: подводятся ВСЕ
    // сезоны, которым пора.
    {
        let svc = service.clone();
        tokio::spawn(async move {
            let mut tick = tokio::time::interval(std::time::Duration::from_secs(900));
            loop {
                tick.tick().await;
                match svc
                    .track_job("studio_verdict", 900, svc.judge_due_studio_seasons(), |n| {
                        (*n > 0).then(|| format!("seasons judged: {n}"))
                    })
                    .await
                {
                    Ok(n) if n > 0 => tracing::info!("Сезон студии подведён: {n} к хозяину"),
                    Ok(_) => {}
                    Err(e) => tracing::warn!("Вердикт сезона не удался: {e}"),
                }
            }
        });
    }

    // Background: молоток. Тик тот же четвертьчасовой, что и у вердикта, и по
    // той же причине: у торга есть НАЗНАЧЕННАЯ минута конца, а ставка в
    // последние минуты её ещё и отодвигает. Суточный тик закрывал бы лоты
    // когда придётся, и «до конца два часа» на странице значило бы «когда-то».
    {
        let svc = service.clone();
        tokio::spawn(async move {
            let mut tick = tokio::time::interval(std::time::Duration::from_secs(900));
            loop {
                tick.tick().await;
                match svc
                    .track_job("auctions", 900, svc.close_due_auctions(), |n| {
                        (*n > 0).then(|| format!("lots sold: {n}"))
                    })
                    .await
                {
                    Ok(n) if n > 0 => tracing::info!("Молоток: продано лотов {n}"),
                    Ok(_) => {}
                    Err(e) => tracing::warn!("Молоток не удался: {e}"),
                }
            }
        });
    }

    // Background: уборка склада студии — раз в сутки.
    //
    // Суточный тик здесь уместен ровно потому, почему у вердикта он неуместен:
    // мусор не назначен на минуту. Лист, брошенный в среду, может полежать до
    // четверга — и никому от этого не хуже.
    {
        let svc = service.clone();
        tokio::spawn(async move {
            let mut tick = tokio::time::interval(std::time::Duration::from_secs(24 * 3600));
            loop {
                tick.tick().await;
                match svc
                    .track_job("studio_store_sweep", 24 * 3600, svc.sweep_studio_store(), |(sheets, assets)| {
                        (*sheets > 0 || *assets > 0)
                            .then(|| format!("sheets {sheets}, parts {assets} removed"))
                    })
                    .await
                {
                    Ok((0, 0)) => {}
                    Ok((sheets, assets)) => {
                        tracing::info!("Склад студии убран: листов {sheets}, деталей {assets}")
                    }
                    Err(e) => tracing::warn!("Уборка склада студии не удалась: {e}"),
                }
            }
        });
    }

    // Фото гостей к заявкам: загруженное, но не приложенное ни к одной заявке за сутки,
    // стирается вместе со строкой. Раз в сутки; опоздать этот проход не может.
    {
        let svc = service.clone();
        tokio::spawn(async move {
            let mut tick = tokio::time::interval(std::time::Duration::from_secs(24 * 3600));
            loop {
                tick.tick().await;
                match svc
                    .track_job("commission_upload_cleanup", 24 * 3600, svc.sweep_commission_uploads(), |n| {
                        (*n > 0).then(|| format!("photos removed: {n}"))
                    })
                    .await
                {
                    Ok(0) => {}
                    Ok(n) => tracing::info!("Фото к заявкам убраны: {n}"),
                    Err(e) => tracing::warn!("Уборка фото к заявкам не удалась: {e}"),
                }
            }
        });
    }

    // Саморегистрация webhook Telegram: сервер сам говорит, куда стучаться.
    // Фоном — сеть не должна задерживать открытие сайта.
    {
        let svc = service.clone();
        tokio::spawn(async move {
            let _ = svc
                .track_job("telegram_webhook", 0, svc.announce_telegram_webhook(), |done| {
                    done.then(|| "webhook registered with Telegram".into())
                })
                .await;
        });
    }

    // A new binary is a release: say so on the analytics trend. Fire and
    // forget, a little after startup, so it never delays opening the site.
    {
        let svc = service.clone();
        tokio::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_secs(30)).await;
            svc.note_release_if_new().await;
        });
    }

    // Background: поиск по смыслу догоняет текст работ. Сохранение работы
    // обновляет поиск само, но фоном и без повтора; этот проход раз в десять
    // минут дочищает то, что не успело или упало, и убирает снятые с сайта
    // работы. Первый проход — через две минуты после запуска: после выкладки
    // с новой моделью поиск переписывается сам, без кнопки в админке.
    {
        let svc = service.clone();
        tokio::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_secs(120)).await;
            let mut tick = tokio::time::interval(std::time::Duration::from_secs(600));
            loop {
                tick.tick().await;
                match svc
                    .track_job("search_index", 600, svc.refresh_stale_embeddings(), |n| {
                        (*n > 0).then(|| format!("works re-indexed: {n}"))
                    })
                    .await
                {
                    Ok(n) if n > 0 => tracing::info!("Search index: {n} works refreshed"),
                    Ok(_) => {}
                    Err(e) => tracing::warn!("Search index refresh round failed: {e}"),
                }
            }
        });
    }

    // Background: каталог ответов «Спросить сайт» строится при первом вопросе
    // (вектор каждой формулировки), а это секунды работы модели. Строим сразу
    // после запуска, чтобы первый посетитель их не ждал.
    {
        let svc = service.clone();
        tokio::spawn(async move {
            match svc
                .track_job("ask_index_warm", 0, svc.warm_ask_index(), |n| {
                    Some(format!("answers indexed: {n}"))
                })
                .await
            {
                Ok(n) => tracing::info!("Ask the site: {n} answers indexed"),
                Err(e) => tracing::warn!("Ask the site: the catalog was not indexed at start: {e}"),
            }
        });
    }

    // Background: поиск по всему сайту держит небылицы, листы вестника и карты
    // вместе с векторами (`site_search`). Строим при запуске: первый поиск иначе
    // ждал бы модель. Дальше индекс пересобирается сам раз в несколько минут.
    {
        let svc = service.clone();
        tokio::spawn(async move {
            match svc
                .track_job("site_index_warm", 0, svc.warm_site_index(), |n| {
                    Some(format!("tales, leaves and cards indexed: {n}"))
                })
                .await
            {
                Ok(n) => tracing::info!("Site search: {n} tales, leaves and cards indexed"),
                Err(e) => tracing::warn!("Site search: the index was not built at start: {e}"),
            }
        });
    }

    // Background: объявления в Telegram-канале. Тик — пять минут: вместе с
    // выдержкой в десять минут работа выходит в канал через десять–пятнадцать
    // минут после того, как появилась на сайте. Первый тик — через минуту
    // после запуска, чтобы не толкаться с миграциями и прогревом.
    {
        let svc = service.clone();
        tokio::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_secs(60)).await;
            let mut tick = tokio::time::interval(std::time::Duration::from_secs(300));
            loop {
                tick.tick().await;
                // Каналы и уведомления в браузере: что на людях читается один раз на оба.
                match svc
                    .track_job("channel_news", 300, svc.announce_news(), |(posts, pushes)| {
                        (*posts > 0 || *pushes > 0)
                            .then(|| format!("channel posts {posts}, browser notifications {pushes}"))
                    })
                    .await
                {
                    Ok((posts, pushes)) => {
                        if posts > 0 {
                            tracing::info!("Telegram channel: {posts} posts out");
                        }
                        if pushes > 0 {
                            tracing::info!("Push: {pushes} notifications out");
                        }
                    }
                    Err(e) => tracing::warn!("News round failed: {e}"),
                }
                // Новый релиз в файле заметок — черновик поста в админке. После
                // объявлений: журнал уже заметил то, что вышло к выкладке.
                if let Err(e) = svc
                    .track_job("release_draft", 300, svc.prepare_release_draft(), |made| {
                        made.as_ref().map(|v| format!("draft of release {v}"))
                    })
                    .await
                {
                    tracing::warn!("Release draft round failed: {e}");
                }
            }
        });
    }

    // Background: письма и записки в Telegram о вышедших небылицах. Тик тот же, что у канала, и по
    // той же причине: вместе с выдержкой в десять минут письмо уходит через
    // десять–пятнадцать минут после того, как байка вышла на люди.
    {
        let svc = service.clone();
        tokio::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_secs(90)).await;
            let mut tick = tokio::time::interval(std::time::Duration::from_secs(300));
            loop {
                tick.tick().await;
                match svc
                    .track_job("tale_letters", 300, svc.send_tale_letters(), |n| {
                        (*n > 0).then(|| format!("letters sent: {n}"))
                    })
                    .await
                {
                    Ok(n) if n > 0 => tracing::info!("Tale letters: {n} sent"),
                    Ok(_) => {}
                    Err(e) => tracing::warn!("Tale letters round failed: {e}"),
                }
                match svc
                    .track_job("tale_book_notes", 300, svc.send_tale_book_notes(), |n| {
                        (*n > 0).then(|| format!("Telegram notes to the house book: {n}"))
                    })
                    .await
                {
                    Ok(n) if n > 0 => tracing::info!("Book notes: {n} sent to Telegram"),
                    Ok(_) => {}
                    Err(e) => tracing::warn!("Book notes round failed: {e}"),
                }
                match svc
                    .track_job("tale_notes", 300, svc.send_tale_notes(), |n| {
                        (*n > 0).then(|| format!("Telegram notes to those waiting: {n}"))
                    })
                    .await
                {
                    Ok(n) if n > 0 => tracing::info!("Tale notes: {n} sent to Telegram"),
                    Ok(_) => {}
                    Err(e) => tracing::warn!("Tale notes round failed: {e}"),
                }
            }
        });
    }

    // Background: prune login attempts past the retention window (runs now, then daily).
    {
        let svc = service.clone();
        tokio::spawn(async move {
            const RETENTION_DAYS: i64 = 90;
            let mut tick = tokio::time::interval(std::time::Duration::from_secs(24 * 3600));
            loop {
                tick.tick().await;
                match svc
                    .track_job("prune_login_history", 24 * 3600, svc.prune_login_history(RETENTION_DAYS), |n| {
                        (*n > 0).then(|| format!("login attempts removed: {n}"))
                    })
                    .await
                {
                    Ok(n) if n > 0 => {
                        tracing::info!("Pruned {n} login attempts older than {RETENTION_DAYS} days")
                    }
                    Ok(_) => {}
                    Err(e) => tracing::warn!("Login-attempt prune failed: {e}"),
                }
                // Отработавшие и остывшие слова для входа через Telegram —
                // здесь же: тот же срок жизни у задачи, та же уборка за входом.
                match svc
                    .track_job("prune_telegram_login", 24 * 3600, svc.prune_telegram_login_codes(), |n| {
                        (*n > 0).then(|| format!("Telegram login words removed: {n}"))
                    })
                    .await
                {
                    Ok(n) if n > 0 => tracing::info!("Pruned {n} spent Telegram login words"),
                    Ok(_) => {}
                    Err(e) => tracing::warn!("Telegram login-word prune failed: {e}"),
                }
                // Просроченные сессии — все разом. Прежде сессия стиралась только при
                // следующем входе того же человека, и у не вернувшегося её строка
                // (с адресом и браузером) оставалась навсегда.
                match svc
                    .track_job("prune_sessions", 24 * 3600, svc.prune_expired_sessions(), |n| {
                        (*n > 0).then(|| format!("expired sessions removed: {n}"))
                    })
                    .await
                {
                    Ok(n) if n > 0 => tracing::info!("Pruned {n} expired sessions"),
                    Ok(_) => {}
                    Err(e) => tracing::warn!("Expired-session prune failed: {e}"),
                }
                // Гости боя: имя без дверей живёт месяц, потом уходит с партиями.
                match svc
                    .track_job("prune_guests", 24 * 3600, svc.prune_stale_guests(), |n| {
                        (*n > 0).then(|| format!("stale guest players removed: {n}"))
                    })
                    .await
                {
                    Ok(n) if n > 0 => tracing::info!("Pruned {n} stale guest players"),
                    Ok(_) => {}
                    Err(e) => tracing::warn!("Guest-player prune failed: {e}"),
                }
                // Отработавшие и остывшие попытки входа через Google и Apple.
                match svc
                    .track_job("prune_oauth", 24 * 3600, svc.prune_oauth_logins(), |n| {
                        (*n > 0).then(|| format!("spent OAuth sign-ins removed: {n}"))
                    })
                    .await
                {
                    Ok(n) if n > 0 => tracing::info!("Pruned {n} spent OAuth sign-ins"),
                    Ok(_) => {}
                    Err(e) => tracing::warn!("OAuth sign-in prune failed: {e}"),
                }
            }
        });
    }

    // Background: prune raw analytics events in small chunks so retention never
    // creates a large delete transaction on the small production server.
    {
        let svc = service.clone();
        tokio::spawn(async move {
            const RETENTION_DAYS: i64 = gotiga_server::analytics::RETENTION_DAYS;
            const BATCH_SIZE: i64 = 1000;
            let mut tick = tokio::time::interval(std::time::Duration::from_secs(24 * 3600));
            loop {
                tick.tick().await;
                let round = svc
                    .track_job(
                        "analytics_prune",
                        24 * 3600,
                        async {
                            let mut total: u64 = 0;
                            loop {
                                match svc
                                    .prune_analytics_events_chunked(RETENTION_DAYS, BATCH_SIZE)
                                    .await?
                                {
                                    0 => break Ok(total),
                                    n => {
                                        total += n as u64;
                                        tracing::info!(
                                            "Pruned {n} analytics events older than {RETENTION_DAYS} days"
                                        );
                                        tokio::task::yield_now().await;
                                    }
                                }
                            }
                        },
                        |total| (*total > 0).then(|| format!("raw events removed: {total}")),
                    )
                    .await;
                if let Err(e) = round {
                    tracing::warn!("Analytics retention prune failed: {e}");
                }
            }
        });
    }

    // Background: pull the world desk (RSS cuttings) on boot, then every six hours.
    // Failures stay on the feed row; the blotter simply shows what it has.
    {
        let svc = service.clone();
        tokio::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_secs(20)).await;
            match svc
                .track_job("gazette_desk", 6 * 3600, svc.refresh_gazette_desk(), desk_note)
                .await
            {
                Ok(r) if r.imported > 0 => {
                    tracing::info!(
                        "Gazette desk: {} cuttings from {} feeds",
                        r.imported,
                        r.feeds
                    )
                }
                Ok(_) => {}
                Err(e) => tracing::warn!("Gazette desk refresh failed: {e}"),
            }
            let mut tick = tokio::time::interval(std::time::Duration::from_secs(6 * 3600));
            tick.tick().await;
            loop {
                tick.tick().await;
                if let Err(e) = svc
                    .track_job("gazette_desk", 6 * 3600, svc.refresh_gazette_desk(), desk_note)
                    .await
                {
                    tracing::warn!("Gazette desk refresh failed: {e}");
                }
            }
        });
    }

    // Background: sweep the in-memory rate limiters. The per-request check paths
    // only prune the key they touch, so keys for one-off IPs would otherwise
    // accumulate forever — this drops stale/empty ones on a timer.
    {
        let svc = service.clone();
        tokio::spawn(async move {
            let mut tick = tokio::time::interval(std::time::Duration::from_secs(300));
            loop {
                tick.tick().await;
                let removed = svc.prune_rate_limiters().await;
                if removed > 0 {
                    tracing::debug!("Rate-limiter GC dropped {removed} stale keys");
                }
            }
        });
    }

    let router = api::router(service.clone(), config.clone(), log_store);

    // 6. Start Server
    let addr: SocketAddr = format!("{}:{}", config.host, config.port)
        .parse()
        .expect("Invalid address");

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(
        listener,
        router.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown_signal())
    .await?;
    service.shutdown_analytics().await;

    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    tracing::info!("Signal received, starting graceful shutdown");
}

/// Что записать в журнал проходов о подборке заметок мира: сколько вырезок и из скольких лент.
fn desk_note(r: &gotiga_server::models::GazetteRefreshReport) -> Option<String> {
    (r.imported > 0).then(|| format!("cuttings imported: {} from {} feeds", r.imported, r.feeds))
}
