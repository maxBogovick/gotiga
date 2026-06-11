# Commission Requests — План и чеклист

> Запрос на создание **новой** фигурки («прошение мастеру»). Вариант B: отдельная сущность,
> переписка, фото пользователя, расширяемость. Фронтенд (SvelteKit/Tauri) + бэкенд (Rust/Axum).

## 0. Зафиксированные решения

| Вопрос | Решение | Обоснование |
|---|---|---|
| Вход | **Гибрид: гость → потом аккаунт** | Baymard: принудит. регистрация теряет 26–37%; лучшая практика — гость, затем привязка аккаунта после вовлечения |
| Фото в переписке | **Вложения в сообщении** (attachments[]) | «удобно и расширяемо»: фото станут частью любой переписки, не только комишна |
| Объём | **Фронт + Rust целиком** | активная серверная работа в репо |
| Кол-во полей на входе | **3 видимых, остальное прогрессивно/опц.** | HubSpot 40k+: обрыв конверсии после 3 полей; textarea/select бьют сильнее; multi-step +86% |
| Бюджет | **опционально, вилкой** | intake-практика «спросить заранее», но без обязательного поля (эстетика «не-магазина» + конверсия) |
| Рамка/копирайт | **«прошение мастеру», мастер может отклонить** | CLAUDE.md: «NOT a shop» |

### Архитектурная суть
- Новая сущность **`commission`** (может быть гостевой: `user_id` NULL + `claim_token`).
- Первичные референсы прикладываются к самой заявке (`commission_attachments`).
- Переписка — **переиспользуем существующие `message_threads`**, добавив `category = 'commission'`,
  `reference_id = commission.id`. Тред создаётся при первом ответе/после привязки аккаунта
  (т.к. `message_threads.user_id` NOT NULL → переписка живёт у залогиненного пользователя).
- Вложения в сообщениях — расширяем `thread_messages` таблицей `thread_message_attachments`
  (даёт фото в **любой** переписке: booking/order/commission).
- Гость отправляет заявку публично; получает `claim_token` (как `cancel_token` у бронирований);
  на экране успеха предлагаем «войти/создать аккаунт, чтобы следить за ответом» → привязка
  `commission.user_id` и старт треда.

---

## Фаза 1 — Данные (миграции) · `src-tauri/server/migrations/`

- [ ] **M1. `*_commissions.sql`** — таблица `commissions`:
  - `id UUID PK`, `user_id UUID NULL REFERENCES users(id) ON DELETE SET NULL`,
  - `claim_token TEXT UNIQUE NOT NULL` (гостевой доступ),
  - контакт: `requester_name TEXT`, `requester_email TEXT NOT NULL`, `requester_phone TEXT NULL`,
  - суть: `title TEXT`, `description TEXT NOT NULL`,
  - детали (опц.): `size_note TEXT NULL`, `mood TEXT NULL`, `deadline DATE NULL`,
    `budget_note TEXT NULL`, `occasion TEXT NULL`,
  - связь: `figurine_id TEXT NULL` (когда мастер берёт заказ → связь с реальной фигуркой),
  - `status commission_status NOT NULL DEFAULT 'new'`,
  - `admin_notes TEXT NULL`, `created_at`, `updated_at`.
  - ENUM `commission_status`: `new | reviewing | accepted | in_progress | completed | declined`.
  - Индексы: `status`, `created_at DESC`, `user_id`, `claim_token`.
- [ ] **M2. `*_commission_attachments.sql`** — первичные референсы заявки:
  - `id UUID PK`, `commission_id UUID REFERENCES commissions(id) ON DELETE CASCADE`,
  - `url TEXT`, `thumb_url TEXT NULL`, `created_at`. Индекс по `commission_id`.
- [ ] **M3. `*_thread_message_attachments.sql`** — вложения в сообщениях (расширяемо для всех тредов):
  - `id UUID PK`, `message_id UUID REFERENCES thread_messages(id) ON DELETE CASCADE`,
  - `url TEXT`, `thumb_url TEXT NULL`, `created_at`. Индекс по `message_id`.
- [ ] **M4.** Расширить ENUM `category` тредов комментарием `commission` (текстовое поле — менять схему не надо, добавить в код-валидацию).

## Фаза 2 — Бэкенд модели · `src-tauri/server/src/models/mod.rs`

- [ ] **B1.** `CommissionRequest` (вход): контакт + `title/description` + опц. детали + `attachment_urls: Vec<String>`.
- [ ] **B2.** `Commission`, `CommissionDto`, `CommissionsPage`, `CommissionStatus`, `CommissionAttachmentDto`.
- [ ] **B3.** Расширить `ThreadMessageDto` полем `attachments: Vec<AttachmentDto>`; `ReplyRequest` — `attachment_urls: Vec<String>`.
- [ ] **B4.** `UpdateCommissionStatusRequest { status, admin_notes?, figurine_id? }`.
- [ ] **B5.** `ClaimCommissionRequest`/ответ для привязки гостевой заявки к аккаунту.

## Фаза 3 — Бэкенд БД-слой · `src-tauri/server/src/db/mod.rs`

- [ ] **B6.** `create_commission` (+ attachments в транзакции, генерация `claim_token`).
- [ ] **B7.** `get_commission_by_token`, `get_commission_by_id`, `list_commissions(status,page)`, `count_new`.
- [ ] **B8.** `update_commission_status` (+ при `accepted/in_progress` опц. связать `figurine_id`).
- [ ] **B9.** `claim_commission(token, user_id)` → проставить `user_id`, создать `message_thread`
  (`category='commission'`, `reference_id=commission.id`).
- [ ] **B10.** Расширить чтение/запись сообщений: грузить и сохранять `thread_message_attachments`.
- [ ] **B11.** `user_commissions(user_id)` — для кабинета пользователя.

## Фаза 4 — Бэкенд хэндлеры/роуты · `handlers.rs` + `api/mod.rs`

Публичные:
- [ ] **B12.** `POST /commissions` → `create_commission` (гость). Ответ: `{ id, claim_token }`.
- [ ] **B13.** `GET  /commissions/:token` → статус заявки по гостевому токену.
Пользовательские (Bearer sessionToken):
- [ ] **B14.** `POST /profile/commissions/claim` → `claim_commission` (привязка + старт треда).
- [ ] **B15.** `GET  /profile/commissions` → список заявок пользователя.
- [ ] **B16.** `POST /profile/uploads` → **новый аутентиф. эндпоинт загрузки** для юзера
  (сейчас `/upload` ходит под админ-ключом; нужен аналог под `sessionToken`, по образцу `user_upload_avatar`).
  Переиспользует `save_image_variants` → `{ url, thumbUrl }`.
- [ ] **B17.** Расширить `reply_to_thread`/`user_create_thread` приёмом `attachment_urls`.
Админские (admin key):
- [ ] **B18.** `GET  /admin/commissions` (фильтр status, пагинация, `newCount`).
- [ ] **B19.** `PATCH /admin/commissions/:id` → статус/заметки/связь с фигуркой.
- [ ] **B20.** Переписку админ ведёт через уже существующие `/admin/threads*` (ничего нового).

Безопасность публичных эндпоинтов (**забытое — добавить**):
- [ ] **B21.** Honeypot-поле в `CommissionRequest` (скрытое; заполнено → тихо 200, не сохранять).
- [ ] **B22.** Rate-limit на `POST /commissions` и `POST /profile/uploads` (по IP/сессии).
- [ ] **B23.** Валидация: email, лимиты длины, кол-во/размер/MIME вложений.
- [ ] **B24.** SMTP-уведомление мастеру о новой заявке (по образцу orders, если есть).

## Фаза 5 — Фронтенд API · `src/lib/types/api.ts` + `src/lib/api.ts`

- [ ] **F1.** Типы: `CommissionRequest`, `CommissionDto`, `CommissionsPage`, `CommissionStatus`,
  `CommissionAttachmentDto`; расширить `ThreadMessageDto.attachments`, `category` += `'commission'`.
- [ ] **F2.** `api.submitCommission(req)` → `POST /commissions`.
- [ ] **F3.** `api.getCommissionByToken(token)`, `api.claimCommission(sessionToken, claimToken)`.
- [ ] **F4.** `api.getUserCommissions(sessionToken)`.
- [ ] **F5.** `api.uploadUserMedia(sessionToken, file)` → `POST /profile/uploads` (FormData, Bearer).
- [ ] **F6.** Расширить `replyToThread`/`createThread` передачей `attachmentUrls`.
- [ ] **F7.** Админ: `api.adminListCommissions(...)`, `api.updateCommissionStatus(...)`.
- [ ] **F8.** Tauri-ветки (`isTauri`) для новых методов, если затрагивают desktop.

## Фаза 6 — Фронтенд UI

- [ ] **F9.** `/commission` маршрут · `src/routes/commission/+page.svelte` — многошаговая форма-ритуал:
  - Шаг 1 «Идея»: `title` + `description` (textarea).
  - Шаг 2 «Детали и референсы»: размер, настроение, срок, бюджет (опц.), **загрузка фото** (drag&drop, превью).
  - Шаг 3 «Контакт»: имя/email/телефон (у залогиненного — из `authStore`, поля скрыты).
  - Прогресс-индикатор; эстетика пергамента/свитка; honeypot-поле.
- [ ] **F10.** Экран успеха: восковая печать + текст «мастер прочтёт, может ответить — или нет» +
  CTA «войти/создать аккаунт, чтобы следить за ответом» (привязка по `claim_token`).
- [ ] **F11.** Компонент `MessageAttachments.svelte` — превью/лайтбокс вложений в сообщениях
  (переиспользуем в booking/order тредах тоже).
- [ ] **F12.** В UI треда (кабинет пользователя) — кнопка «прикрепить фото» к ответу
  (`uploadUserMedia` → `attachmentUrls`).
- [ ] **F13.** Кабинет пользователя — раздел «Мои прошения» (`getUserCommissions`, статусы).
- [ ] **F14.** Точка входа: **пустое состояние `/upcoming`** («сейчас в работе ничего нет — предложите идею»)
  + ненавязчивая ссылка в подвале/на главной. (Не делать «магазинных» кнопок.)
- [ ] **F15.** Админ-вкладка «Прошения» (`src/lib/components/admin/`) — список, фильтр по статусу,
  смена статуса/заметки, связь с фигуркой, переход в тред переписки.

## Фаза 7 — i18n · `src/lib/i18n/en.ts` (источник) + `ru.ts`

- [ ] **F16.** Ключи формы (шаги, поля, плейсхолдеры, кнопки), экран успеха, статусы комишна,
  тексты вложений, ошибки валидации/спама. Добавить в **оба** файла.

## Фаза 8 — Проверка / Definition of Done

- [ ] **V1.** `cargo build` + `cargo test` зелёные; миграции применяются на чистой БД.
- [ ] **V2.** Гость отправляет заявку с 1–3 фото → запись + attachments в БД, приходит `claim_token`.
- [ ] **V3.** Привязка по `claim_token` под аккаунтом → `user_id` проставлен, тред создан.
- [ ] **V4.** Переписка пользователь↔админ с фото в обе стороны; вложения рендерятся.
- [ ] **V5.** Админ меняет статус new→…→accepted, связывает с фигуркой; `newCount` корректен.
- [ ] **V6.** Honeypot и rate-limit срабатывают; невалидные файлы отклоняются.
- [ ] **V7.** Эстетика: пергамент/печать/шрифты; нигде нет «купить/цена/корзина».
- [ ] **V8.** i18n полон в en+ru; `npm run build` без ошибок типов.
- [ ] **V9.** `/code-review` по диффу.

---

## Открытые микро-решения (есть дефолты, можно не блокироваться)
- Лимит вложений: **дефолт 5 файлов, ≤8 МБ, image/* (jpeg/png/webp)**.
- Срок жизни `claim_token`: **бессрочный** (как `cancel_token` бронирований).
- Гостю на email слать копию `claim_token`-ссылки: **да, если SMTP настроен**.
- Связь принятого комишна с фигуркой: ручная (админ вводит `figurine_id`) на этапе 1.

## Порядок исполнения
Фаза 1 → 2 → 3 → 4 (вкл. безопасность) → 5 → 6 → 7 → 8.
Каждую фазу коммитить отдельно (`commission: <phase>`), ветка от `master`.
