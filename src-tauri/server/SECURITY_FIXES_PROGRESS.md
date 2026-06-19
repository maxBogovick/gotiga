# Security & Quality Fixes — Progress

Статусы: ⬜ todo · 🔄 in progress · ✅ done · ❌ won't fix (с обоснованием)

## 0. Разблокировать CI (делать первым)
- ✅ T0.1 `tests/api_tests.rs` переписан: убран SqlitePool/мёртвый release-тест, Config со всеми полями, smoke-тест health+figurines
- ✅ T0.2 `cargo test --no-run` и `cargo clippy --all-targets -- -D warnings` — exit 0

## 1. Критично (security)
- ✅ T1.1 `main.rs:23` — не логировать `database_url`
- ✅ T1.2 `config.rs:36-37` — fail-fast на дефолтных `admin`/`123`; убрать слабые дефолты (validate() + FORBIDDEN_SECRETS)
- ✅ T1.3 `docker-compose.yml` — секреты только из env (`${VAR:?}`), убран change_me_in_prod
- ✅ T1.4 CORS — whitelist origins из config.cors_allowed_origins
- ✅ T1.5 Статика — отдаём только images/videos/audio/backgrounds/avatars
- ✅ T1.6 body limit — глобально 16MB, media upload routes 256MB
- ✅ T1.7 guard'ы размера: image 40MB+50MP, avatar 10MB до decode
- ✅ T1.8 image decode/resize/encode вынесены в `spawn_blocking` + лимит пикселей
- ✅ T1.9 create_booking_atomic: advisory lock + check + insert в одной транзакции
- ✅ T1.10 confirm_booking_atomic: advisory lock + check + update booking/figurine в транзакции
- ✅ T1.11 save_figurine_full: upsert+images+steps в одной транзакции, UUID парсятся до записи
- ✅ T1.12 `db/mod.rs:736` — токены 64 бита (XXXX-XXXX-XXXX-XXXX)
- ✅ T1.13 rate limit: token(60/ч), booking/order/waitlist(15/ч), login(20-30/ч), register(5/ч), reset(10/ч)
- ✅ T1.14 login enumeration — challenge всегда выдаётся, blocked → generic Unauthorized в verify
- ✅ T1.15 link_bookings_to_user — проверка совпадения email аккаунта

## 2. Высокий приоритет
- ✅ T2.1 Вырезан release/sync/assets flow (routes, handlers, repo, models, resolve_url)
- ⬜ T2.2 `db/mod.rs:1503` — waitlist dedupe: unique index + ON CONFLICT
- ✅ T2.3 booking rules вынесены в validate_booking_rules, применяются и в create_booking, и в reschedule
- ✅ T2.4 `db/mod.rs:866` — прямое присваивание, можно очищать admin_notes/curator_conditions
- ✅ T2.5 invalid commission deadline → 400 (parse_optional_deadline)
- ✅ T2.6 related_item_ids убран из SaveFigurineRequest
- ✅ T2.7 битый JSON настроек → AppError::Internal (parse_json_setting + repo)
- ✅ T2.8 admin_get_threads — bind-параметры
- ✅ T2.9 create_thread/add_thread_reply + message + attachments в транзакции

## 3. Средний приоритет
- ✅ T3.1 N+1 устранён: get_face_images_for_figurines (batch) в list/in-progress/related
- ✅ T3.2 `&m[0..4]` UTF-8 panic → chars().take(4)
- ✅ T3.3 Валидация DTO: validate_attachments/validate_text, лимиты thread/waitlist/figurine/theme/copy
- ✅ T3.4 rate limit расширен на register/login/order/booking/waitlist (см. T1.13)
- ✅ T3.5 `.dockerignore` + non-root user в Dockerfile
- ✅ T3.6 Миграции — CHECK constraints (bookings/showings даты; commissions длины), NOT VALID

## Финальная проверка
- ✅ TF.1 `cargo check`, `cargo test --no-run`, `cargo clippy --all-targets -- -D warnings` — все exit 0

## Дополнительно
- ✅ Создан `src-tauri/server/.env.example` с обязательными секретами и ALLOWED_ORIGINS
- ⚠️ Заметки: рассмотреть persistent rate limiter (сейчас in-memory, сбрасывается при рестарте) и доверие x-forwarded-for (валидно только за доверенным прокси). ON CONFLICT для notify-orders (упрощённый upsert, не флагнут ревью).
