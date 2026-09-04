//! Unit tests for the pure, in-process logic of the service layer.
//!
//! These cover the deterministic helpers that carry real decisions —
//! credential hashing, the anti-enumeration decoy pool, pool/selection
//! validation, booking-rule arithmetic, attachment/text validation, URL
//! resolution and the Telegram MarkdownV2 escaper. None of them touch
//! Postgres, so they run in milliseconds and are fully reproducible.
//!
//! DB-backed like/wishlist behaviour is in `tests/figurine_likes.rs`.

use super::*;
use crate::config::Config;

// ── Test fixtures ───────────────────────────────────────────────────────────

/// A `Config` with a stable `public_url` / `admin_api_key` for URL- and
/// decoy-pool assertions. No network or DB is touched.
fn test_config() -> Config {
    Config {
        database_url: "postgres://unused/unused".into(),
        host: "127.0.0.1".into(),
        port: 0,
        admin_api_key: "secret-key-for-decoys".into(),
        upload_dir: "/tmp/gotiga-test-uploads".into(),
        public_url: "https://gotiga.example".into(),
        rust_log: String::new(),
        admin_login: "admin".into(),
        admin_password: "pw".into(),
        cors_allowed_origins: vec![],
        telegram_bot_token: None,
        telegram_chat_id: None,
        smtp_host: None,
        smtp_port: None,
        smtp_user: None,
        smtp_pass: None,
        smtp_from: None,
        geoip_db_path: None,
        admin_log_db_path: "/tmp/gotiga-test-admin-logs.sqlite".into(),
        analytics_hash_secret: "analytics-secret-for-tests".into(),
    }
}

/// Build an `AppService` whose pool is *lazy* — it never connects to Postgres.
/// Safe to use only for methods that read `self.config` and never touch the DB
/// (URL resolution, media path cleaning, …).
fn lazy_service() -> AppService {
    let pool = sqlx::postgres::PgPoolOptions::new()
        .connect_lazy("postgres://unused:unused@127.0.0.1:1/unused")
        .expect("connect_lazy never dials");
    AppService::new(Repository::new(pool), test_config())
}

/// A valid 8-icon subset for one category, drawn from the master ICONS pool,
/// with `selected` guaranteed to be a member.
fn valid_category(index: usize) -> Vec<String> {
    let ids = valid_icon_ids(CATEGORIES[index]).unwrap();
    ids.iter()
        .take(POOL_PER_CATEGORY)
        .map(|s| s.to_string())
        .collect()
}

/// A complete, valid `(pool, selections)` pair for registration.
fn valid_pool_and_selections() -> ([Vec<String>; 4], [String; 4]) {
    let pool = [
        valid_category(0),
        valid_category(1),
        valid_category(2),
        valid_category(3),
    ];
    let selections = [
        pool[0][0].clone(),
        pool[1][1].clone(),
        pool[2][2].clone(),
        pool[3][3].clone(),
    ];
    (pool, selections)
}

fn assert_bad_request(err: AppError, needle: &str) {
    match err {
        AppError::BadRequest(msg) => assert!(
            msg.contains(needle),
            "expected BadRequest containing {needle:?}, got: {msg:?}"
        ),
        other => panic!("expected BadRequest({needle:?}), got {other:?}"),
    }
}

// ── validate_text ───────────────────────────────────────────────────────────

#[test]
fn validate_text_rejects_blank_and_whitespace_only() {
    assert_bad_request(validate_text("Name", "", 10).unwrap_err(), "required");
    assert_bad_request(
        validate_text("Name", "   \t\n", 10).unwrap_err(),
        "required",
    );
}

#[test]
fn validate_text_counts_unicode_scalars_not_bytes() {
    // "приве́т" style: 6 multi-byte chars must pass a max of 6 even though the
    // byte length is far larger.
    assert!(validate_text("Name", "приве́", 6).is_ok());
    // 7 chars over a max of 6 fails — proves it counts chars, not bytes.
    assert_bad_request(validate_text("Name", "abcdefg", 6).unwrap_err(), "too long");
}

#[test]
fn validate_text_trims_before_measuring() {
    // 3 real chars padded with spaces still fits a max of 3.
    assert!(validate_text("Name", "  abc  ", 3).is_ok());
}

// ── validate_attachments ────────────────────────────────────────────────────

fn att(url: &str) -> AttachmentInput {
    AttachmentInput {
        url: url.into(),
        thumb_url: None,
    }
}

#[test]
fn validate_attachments_accepts_allowed_schemes() {
    let ok = [
        att("/static/x.png"),
        att("http://a/b.png"),
        att("https://a/b.png"),
    ];
    assert!(validate_attachments(&ok).is_ok());
}

#[test]
fn validate_attachments_rejects_foreign_scheme_and_relative() {
    assert_bad_request(
        validate_attachments(&[att("ftp://a/b")]).unwrap_err(),
        "Invalid attachment URL",
    );
    assert_bad_request(
        validate_attachments(&[att("javascript:alert(1)")]).unwrap_err(),
        "Invalid attachment URL",
    );
    assert_bad_request(
        validate_attachments(&[att("images/x.png")]).unwrap_err(),
        "Invalid attachment URL",
    );
}

#[test]
fn validate_attachments_validates_thumbnail_too() {
    let bad_thumb = AttachmentInput {
        url: "/static/ok.png".into(),
        thumb_url: Some("ftp://evil".into()),
    };
    assert_bad_request(validate_attachments(&[bad_thumb]).unwrap_err(), "thumbnail");
}

#[test]
fn validate_attachments_enforces_count_cap() {
    let many: Vec<AttachmentInput> = (0..MAX_ATTACHMENTS + 1)
        .map(|_| att("/static/x.png"))
        .collect();
    assert_bad_request(validate_attachments(&many).unwrap_err(), "Too many");
    // Exactly the cap is allowed.
    let at_cap: Vec<AttachmentInput> = (0..MAX_ATTACHMENTS).map(|_| att("/static/x.png")).collect();
    assert!(validate_attachments(&at_cap).is_ok());
}

#[test]
fn validate_attachments_rejects_overlong_url() {
    let long = format!("https://a/{}", "x".repeat(MAX_URL_LEN));
    assert_bad_request(validate_attachments(&[att(&long)]).unwrap_err(), "Invalid");
}

// ── parse_json_setting ──────────────────────────────────────────────────────

#[test]
fn parse_json_setting_missing_yields_default() {
    let s: SmtpSettings = parse_json_setting("smtp", None).unwrap();
    assert_eq!(s.host, None);
    assert_eq!(s.port, None);
}

#[test]
fn parse_json_setting_roundtrips_valid_json() {
    let s: SmtpSettings =
        parse_json_setting("smtp", Some(r#"{"host":"mail","port":25}"#.into())).unwrap();
    assert_eq!(s.host.as_deref(), Some("mail"));
    assert_eq!(s.port, Some(25));
}

#[test]
fn parse_json_setting_corrupt_is_hard_error_not_silent_default() {
    // The whole point of this helper: corrupt persisted settings must NOT be
    // silently reset to defaults (which would wipe SMTP creds / theme).
    let err = parse_json_setting::<SmtpSettings>("smtp", Some("{not json".into())).unwrap_err();
    match err {
        AppError::Internal(msg) => assert!(msg.contains("Corrupt setting 'smtp'")),
        other => panic!("expected Internal corrupt-setting error, got {other:?}"),
    }
}

// ── escape_markdown ─────────────────────────────────────────────────────────

#[test]
fn escape_markdown_escapes_every_v2_special_char() {
    let specials = "_*[]()~`>#+-=|{}.!";
    let escaped = escape_markdown(specials);
    // Each special char is preceded by a backslash → output is exactly double.
    assert_eq!(escaped.len(), specials.len() * 2);
    for c in specials.chars() {
        assert!(escaped.contains(&format!("\\{c}")));
    }
}

#[test]
fn escape_markdown_leaves_plain_text_untouched() {
    assert_eq!(escape_markdown("Hello мир 123"), "Hello мир 123");
}

// ── decoy_pool (anti-enumeration) ───────────────────────────────────────────

#[test]
fn decoy_pool_is_deterministic_for_same_email_and_secret() {
    let a = decoy_pool("secret", "user@example.com");
    let b = decoy_pool("secret", "user@example.com");
    assert_eq!(
        a, b,
        "decoy grid must be stable so it can't be probed for liveness"
    );
}

#[test]
fn decoy_pool_has_correct_shape() {
    let p = decoy_pool("secret", "user@example.com");
    assert_eq!(p.len(), CATEGORIES.len());
    for (i, cat) in p.iter().enumerate() {
        assert_eq!(cat.len(), POOL_PER_CATEGORY, "category {i} wrong size");
        // No duplicates within a category.
        let unique: std::collections::HashSet<&String> = cat.iter().collect();
        assert_eq!(unique.len(), POOL_PER_CATEGORY);
        // Every icon is a real master-pool id for that category.
        let master = valid_icon_ids(CATEGORIES[i]).unwrap();
        for id in cat {
            assert!(
                master.contains(&id.as_str()),
                "{id} not a valid {}",
                CATEGORIES[i]
            );
        }
    }
}

#[test]
fn decoy_pool_differs_by_email_and_by_secret() {
    let base = decoy_pool("secret", "a@example.com");
    assert_ne!(
        base,
        decoy_pool("secret", "b@example.com"),
        "must vary per email"
    );
    assert_ne!(
        base,
        decoy_pool("other-secret", "a@example.com"),
        "must be keyed by the admin secret so it can't be recomputed"
    );
}

// ── validate_pool ───────────────────────────────────────────────────────────

#[test]
fn validate_pool_accepts_well_formed_pool_and_returns_keyed_json() {
    let (pool, selections) = valid_pool_and_selections();
    let json = validate_pool(&pool, &selections).unwrap();
    let obj = json.as_object().unwrap();
    for cat in CATEGORIES {
        let arr = obj.get(cat).expect("category present").as_array().unwrap();
        assert_eq!(arr.len(), POOL_PER_CATEGORY);
    }
}

#[test]
fn validate_pool_rejects_wrong_size() {
    let (mut pool, selections) = valid_pool_and_selections();
    pool[0].pop(); // now 7
    assert_bad_request(
        validate_pool(&pool, &selections).unwrap_err(),
        "Invalid pool size",
    );
}

#[test]
fn validate_pool_rejects_foreign_icon() {
    let (mut pool, selections) = valid_pool_and_selections();
    pool[1][0] = "not_a_real_icon".into();
    assert_bad_request(
        validate_pool(&pool, &selections).unwrap_err(),
        "Invalid pool icon",
    );
}

#[test]
fn validate_pool_rejects_duplicates() {
    let (mut pool, selections) = valid_pool_and_selections();
    pool[2][1] = pool[2][0].clone();
    assert_bad_request(
        validate_pool(&pool, &selections).unwrap_err(),
        "Duplicate pool icon",
    );
}

#[test]
fn validate_pool_rejects_selection_outside_pool() {
    let (pool, mut selections) = valid_pool_and_selections();
    // Pick a valid icon for that category that is NOT in the shown subset.
    let master = valid_icon_ids(CATEGORIES[0]).unwrap();
    let outside = master
        .iter()
        .find(|id| !pool[0].contains(&id.to_string()))
        .unwrap();
    selections[0] = outside.to_string();
    assert_bad_request(
        validate_pool(&pool, &selections).unwrap_err(),
        "Selection not in pool",
    );
}

// ── parse_stored_pool ───────────────────────────────────────────────────────

#[test]
fn parse_stored_pool_roundtrips_validate_pool_output() {
    let (pool, selections) = valid_pool_and_selections();
    let json = validate_pool(&pool, &selections).unwrap();
    let parsed = parse_stored_pool(&json).expect("valid stored pool parses");
    assert_eq!(parsed, pool.to_vec());
}

#[test]
fn parse_stored_pool_rejects_missing_category_or_wrong_shape() {
    // Missing a category.
    let mut obj = serde_json::Map::new();
    obj.insert("animals".into(), serde_json::json!(["wolf"]));
    assert!(parse_stored_pool(&serde_json::Value::Object(obj)).is_none());
    // Not an object at all.
    assert!(parse_stored_pool(&serde_json::json!(["x"])).is_none());
    // Category present but empty → unusable.
    let empty = serde_json::json!({
        "animals": [], "dishes": [], "seasons": [], "symbols": []
    });
    assert!(parse_stored_pool(&empty).is_none());
}

// ── build_hash_input / hashing ──────────────────────────────────────────────

#[test]
fn build_hash_input_is_category_labelled_and_ordered() {
    let sel = [
        "wolf".to_string(),
        "apple".to_string(),
        "sun".to_string(),
        "key".to_string(),
    ];
    assert_eq!(
        build_hash_input(&sel),
        "animals:wolf|dishes:apple|seasons:sun|symbols:key"
    );
}

#[test]
fn build_hash_input_distinguishes_same_icons_in_different_categories() {
    // Two selection arrays sharing icon strings but in different category slots
    // must hash-input differently, so a credential can't be replayed cross-slot.
    let a = [
        "x".to_string(),
        "y".to_string(),
        "z".to_string(),
        "w".to_string(),
    ];
    let b = [
        "y".to_string(),
        "x".to_string(),
        "z".to_string(),
        "w".to_string(),
    ];
    assert_ne!(build_hash_input(&a), build_hash_input(&b));
}

#[test]
fn hash_password_verifies_and_rejects_wrong_input() {
    let hash = hash_password("animals:wolf|dishes:apple|seasons:sun|symbols:key").unwrap();
    assert!(verify_password(
        "animals:wolf|dishes:apple|seasons:sun|symbols:key",
        &hash
    ));
    assert!(!verify_password(
        "animals:fox|dishes:apple|seasons:sun|symbols:key",
        &hash
    ));
}

#[test]
fn hash_password_is_salted_so_two_hashes_differ() {
    let a = hash_password("same-input").unwrap();
    let b = hash_password("same-input").unwrap();
    assert_ne!(a, b, "argon2 salt must randomise the stored hash");
    assert!(verify_password("same-input", &a) && verify_password("same-input", &b));
}

#[test]
fn verify_password_returns_false_on_malformed_hash() {
    assert!(!verify_password("anything", "not-a-phc-hash"));
}

// ── validate_booking_rules ──────────────────────────────────────────────────

fn d(s: &str) -> chrono::NaiveDate {
    chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d").unwrap()
}

#[test]
fn booking_rules_duration_is_inclusive_of_both_endpoints() {
    let rules = BookingRules {
        min_days: 1,
        max_days: 30,
        advance_days: 0,
    };
    // Same start/end day = 1-day booking, must satisfy min_days = 1.
    assert!(AppService::validate_booking_rules(&rules, d("2030-01-10"), d("2030-01-10")).is_ok());
}

#[test]
fn booking_rules_rejects_below_min_and_above_max() {
    let rules = BookingRules {
        min_days: 2,
        max_days: 3,
        advance_days: 0,
    };
    assert_bad_request(
        AppService::validate_booking_rules(&rules, d("2030-01-10"), d("2030-01-10")).unwrap_err(),
        "Minimum booking duration",
    );
    // 4 inclusive days > max 3.
    assert_bad_request(
        AppService::validate_booking_rules(&rules, d("2030-01-10"), d("2030-01-13")).unwrap_err(),
        "Maximum booking duration",
    );
}

#[test]
fn booking_rules_enforces_advance_notice() {
    let rules = BookingRules {
        min_days: 1,
        max_days: 30,
        advance_days: 7,
    };
    // Starting tomorrow violates a 7-day advance requirement.
    let tomorrow = chrono::Utc::now().date_naive() + chrono::Duration::days(1);
    assert_bad_request(
        AppService::validate_booking_rules(&rules, tomorrow, tomorrow).unwrap_err(),
        "in advance",
    );
    // Starting comfortably past the window is fine.
    let far = chrono::Utc::now().date_naive() + chrono::Duration::days(30);
    assert!(AppService::validate_booking_rules(&rules, far, far).is_ok());
}

// ── parse_uuid ──────────────────────────────────────────────────────────────

#[test]
fn parse_uuid_accepts_valid_and_reports_the_bad_value() {
    let id = Uuid::new_v4();
    assert_eq!(AppService::parse_uuid(&id.to_string()).unwrap(), id);
    assert_bad_request(
        AppService::parse_uuid("nope").unwrap_err(),
        "Invalid ID: nope",
    );
}

// ── media path classification ───────────────────────────────────────────────

#[test]
fn is_managed_media_path_only_matches_managed_roots() {
    for p in [
        "images/a.png",
        "videos/a.mp4",
        "audio/a.mp3",
        "backgrounds/b.png",
    ] {
        assert!(
            AppService::is_managed_media_path(p),
            "{p} should be managed"
        );
    }
    for p in ["uploads/x", "static/x", "x.png", ""] {
        assert!(
            !AppService::is_managed_media_path(p),
            "{p} should not be managed"
        );
    }
}

#[test]
fn media_type_for_path_maps_each_root() {
    assert_eq!(AppService::media_type_for_path("images/a.png"), "image");
    assert_eq!(
        AppService::media_type_for_path("backgrounds/a.png"),
        "image"
    );
    assert_eq!(AppService::media_type_for_path("videos/a.mp4"), "video");
    assert_eq!(AppService::media_type_for_path("audio/a.mp3"), "audio");
    assert_eq!(AppService::media_type_for_path("misc/a"), "other");
}

#[test]
fn variant_for_path_recognises_image_variants_only() {
    assert_eq!(
        AppService::variant_for_path("images/original/a.png").as_deref(),
        Some("original")
    );
    assert_eq!(
        AppService::variant_for_path("images/preview/a.png").as_deref(),
        Some("preview")
    );
    assert_eq!(
        AppService::variant_for_path("images/thumb/a.png").as_deref(),
        Some("thumb")
    );
    assert_eq!(AppService::variant_for_path("images/a.png"), None);
    assert_eq!(AppService::variant_for_path("videos/original/a.mp4"), None);
}

// ── URL resolution (needs self.config) ──────────────────────────────────────

#[tokio::test]
async fn resolve_url_passes_through_absolute_http() {
    let svc = lazy_service();
    assert_eq!(
        svc.resolve_url("https://cdn.example/x.png", "images", "id"),
        "https://cdn.example/x.png"
    );
}

#[tokio::test]
async fn resolve_url_prefixes_static_and_relative_against_public_url() {
    let svc = lazy_service();
    assert_eq!(
        svc.resolve_url("/static/images/x.png", "images", "id"),
        "https://gotiga.example/static/images/x.png"
    );
    assert_eq!(
        svc.resolve_url("images/x.png", "images", "id"),
        "https://gotiga.example/static/images/x.png"
    );
}

#[tokio::test]
async fn clean_and_public_media_url_are_inverse_around_public_base() {
    let svc = lazy_service();
    // A full public URL is reduced back to the managed relative path …
    assert_eq!(
        svc.clean_media_path("https://gotiga.example/static/images/a.png"),
        "images/a.png"
    );
    // … and the relative path is expanded back to the same public URL.
    assert_eq!(
        svc.public_media_url("images/a.png"),
        "https://gotiga.example/static/images/a.png"
    );
}

#[tokio::test]
async fn clean_media_path_normalises_backslashes() {
    let svc = lazy_service();
    assert_eq!(
        svc.clean_media_path("images\\sub\\a.png"),
        "images/sub/a.png"
    );
}

// ── in-memory rate limiter ──────────────────────────────────────────────────

#[tokio::test]
async fn check_rate_limit_allows_up_to_max_then_blocks_per_key() {
    let svc = lazy_service();
    // First two requests for (bucket, ip) pass; the third is throttled.
    assert!(
        svc.check_rate_limit("auth", "1.2.3.4", 2, 3600)
            .await
            .is_ok()
    );
    assert!(
        svc.check_rate_limit("auth", "1.2.3.4", 2, 3600)
            .await
            .is_ok()
    );
    assert_bad_request(
        svc.check_rate_limit("auth", "1.2.3.4", 2, 3600)
            .await
            .unwrap_err(),
        "Too many",
    );

    // A different IP has its own independent budget.
    assert!(
        svc.check_rate_limit("auth", "5.6.7.8", 2, 3600)
            .await
            .is_ok()
    );
    // …and so does a different bucket for the same IP.
    assert!(
        svc.check_rate_limit("orders", "1.2.3.4", 2, 3600)
            .await
            .is_ok()
    );
}

#[tokio::test]
async fn set_figurine_like_rejects_empty_and_oversized_token() {
    let svc = lazy_service();
    let id = Uuid::nil();
    assert_bad_request(
        svc.set_figurine_like(id, "  ", None, true)
            .await
            .unwrap_err(),
        "Invalid visitor token",
    );
    assert_bad_request(
        svc.set_figurine_like(id, &"x".repeat(65), None, true)
            .await
            .unwrap_err(),
        "Invalid visitor token",
    );
}

// ── Печать под этюдом ───────────────────────────────────────────────────────
//
// Одно число решает, стоит ли возвращаться к пройденному этюду, — значит оно
// обязано мерить то, что обещает. Здесь проверяется ровно это: что оно мерит
// решения гостя, а не нажатия и не чужие ходы.

/// Расстановка, в которой гость наверняка успевает ударить первым.
fn one_blow_apart() -> battle_core::Setup {
    battle_core::Setup {
        player_board: vec![(
            battle_core::CardSnapshot::new("боец", 1, 9, 9),
            battle_core::Cell::new(1, 3).unwrap(),
        )],
        keeper_board: vec![(
            battle_core::CardSnapshot::new("ворон", 1, 2, 1),
            battle_core::Cell::new(1, 2).unwrap(),
        )],
        ..Default::default()
    }
}

#[test]
fn the_line_counts_decisions_and_not_keystrokes() {
    let setup = one_blow_apart();
    let mut state = battle_core::MatchState::begin(setup.clone());
    let attack = battle_core::Action::Attack {
        attacker: 0,
        target: 1,
    };
    state = battle_core::reduce(&state, &attack).unwrap().0;

    // Один удар и сколько угодно концов хода — это одно дело.
    let journal = vec![
        attack.clone(),
        battle_core::Action::EndTurn,
        battle_core::Action::EndTurn,
    ];
    let (acts, lost) = AppService::count_the_line(&setup, &journal, battle_core::Rules::default());
    assert_eq!(acts, 1, "конец хода — не выбор, а его отсутствие");
    assert_eq!(lost, 0);
}

#[test]
fn the_keepers_own_moves_are_not_the_guests_line() {
    // Партия доигрывается ботом за обе стороны: в журнале оказываются ходы
    // обеих. Печать обязана считать только половину гостя — иначе «за пять
    // дел» значило бы «хранитель был неразговорчив».
    //
    // Расстановка нарочно вязкая: слабые тела вдалеке друг от друга, чтобы
    // хранитель успел походить, а не пал от первого удара.
    let setup = battle_core::Setup {
        player_board: vec![(
            battle_core::CardSnapshot::new("боец", 1, 8, 1),
            battle_core::Cell::new(0, 5).unwrap(),
        )],
        keeper_board: vec![(
            battle_core::CardSnapshot::new("ворон", 1, 8, 1),
            battle_core::Cell::new(2, 0).unwrap(),
        )],
        ..Default::default()
    };

    let mut state = battle_core::MatchState::begin(setup.clone());
    let mut journal = Vec::new();
    // Тот же счёт, но веденный здесь, снаружи: если помощник считает то же
    // самое, он считает правильно, а не «непротиворечиво самому себе».
    let mut expected = 0i16;
    let mut keeper_moved = 0;
    let mut guard = 0;
    while state.outcome.is_none() && guard < 500 {
        let mine = state.active == battle_core::Side::Player;
        let action = battle_core::bot::choose(&state);
        if mine && !matches!(action, battle_core::Action::EndTurn) {
            expected += 1;
        }
        if !mine {
            keeper_moved += 1;
        }
        state = battle_core::reduce(&state, &action).unwrap().0;
        journal.push(action);
        guard += 1;
    }

    assert!(keeper_moved > 0, "хранитель должен был походить");
    let (acts, _) = AppService::count_the_line(&setup, &journal, battle_core::Rules::default());
    assert_eq!(acts, expected);
    assert!((acts as usize) < journal.len());
}

#[test]
fn a_fallen_body_of_the_guest_is_counted_even_though_it_left_the_board() {
    // Павшее тело уходит с доски, но остаётся в списке тел — на этом и держится
    // счёт потерь. Считай его по доске, «без потерь» получал бы каждый.
    let setup = battle_core::Setup {
        player_board: vec![(
            battle_core::CardSnapshot::new("щепка", 1, 1, 0),
            battle_core::Cell::new(1, 3).unwrap(),
        )],
        keeper_board: vec![(
            battle_core::CardSnapshot::new("молот", 1, 9, 9),
            battle_core::Cell::new(1, 2).unwrap(),
        )],
        ..Default::default()
    };
    let mut state = battle_core::MatchState::begin(setup.clone());
    let mut journal = Vec::new();
    let mut guard = 0;
    while state.outcome.is_none() && guard < 500 {
        let action = battle_core::bot::choose(&state);
        state = battle_core::reduce(&state, &action).unwrap().0;
        journal.push(action);
        guard += 1;
    }

    let (_, lost) = AppService::count_the_line(&setup, &journal, battle_core::Rules::default());
    assert_eq!(lost, 1, "щепку снесли, и печати это должно быть видно");
}

#[test]
fn a_line_counted_under_the_wrong_rules_would_stop_short() {
    // Зачем правила вообще передаются в свёртку: под домашними правилами шаг
    // не тратит ход, и тело, прошедшее и ударившее, — два законных дела.
    // Стоит правилам сказать «шаг тратит ход целиком», и второе дело
    // становится незаконным, а свёртка обрывается на нём.
    let setup = battle_core::Setup {
        player_board: vec![(
            battle_core::CardSnapshot::new("боец", 1, 9, 4),
            battle_core::Cell::new(1, 4).unwrap(),
        )],
        keeper_board: vec![(
            battle_core::CardSnapshot::new("ворон", 1, 9, 1),
            battle_core::Cell::new(1, 2).unwrap(),
        )],
        ..Default::default()
    };
    let journal = vec![
        battle_core::Action::Move {
            unit: 0,
            to: battle_core::Cell::new(1, 3).unwrap(),
        },
        battle_core::Action::Attack {
            attacker: 0,
            target: 1,
        },
    ];

    let house = battle_core::Rules::default();
    assert!(!house.walk_spends_turn);
    let (acts, _) = AppService::count_the_line(&setup, &journal, house);
    assert_eq!(acts, 2);

    let strict = battle_core::Rules {
        walk_spends_turn: true,
        ..house
    };
    let (acts, _) = AppService::count_the_line(&setup, &journal, strict);
    assert_eq!(acts, 1, "второе дело под этими правилами незаконно");
}
