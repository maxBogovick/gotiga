-- Уведомления в браузере (Web Push): о новых работах, байках и листах вестника.
--
-- Подписка анонимна и принадлежит устройству: ключ строки — адрес, который выдал
-- браузер (`endpoint`). Имени и почты здесь нет намеренно: уведомления о новом на сайте
-- не личное дело, а вход ради них потерял бы тех, кто подписывается с телефона.
-- Язык и темы лежат в строке: сервер собирает текст на языке читателя и шлёт только
-- то, о чём просили.
--
-- `failures` — подряд не доставленные: пять — и адрес считается мёртвым (браузер
-- переустановили, права сняли), строка стирается. Адрес, который узел рассылки назвал
-- несуществующим (404/410), стирается сразу.
CREATE TABLE IF NOT EXISTS push_subscriptions (
    id           UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    endpoint     TEXT NOT NULL UNIQUE,
    p256dh       TEXT NOT NULL,
    auth         TEXT NOT NULL,
    lang         TEXT NOT NULL DEFAULT 'en'
                 CHECK (lang IN ('en', 'ru', 'uk', 'es', 'de', 'it')),
    topics       TEXT[] NOT NULL
                 CHECK (cardinality(topics) > 0 AND topics <@ ARRAY['works', 'tales', 'gazette']),
    failures     INT NOT NULL DEFAULT 0,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_sent_at TIMESTAMPTZ
);

-- Журнал рассылки: одна строка на вещь, как у канала в Telegram. События «опубликовано»
-- здесь тоже нет — проход раз в пять минут сверяет то, что на людях сейчас, с журналом.
-- Вещь, замеченная впервые, ждёт выдержку (её ещё правят) и уходит один раз.
CREATE TABLE IF NOT EXISTS push_posts (
    kind          TEXT NOT NULL,
    target_id     UUID NOT NULL,
    first_seen_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    sent_at       TIMESTAMPTZ,
    PRIMARY KEY (kind, target_id)
);

CREATE INDEX IF NOT EXISTS idx_push_posts_waiting
    ON push_posts (first_seen_at) WHERE sent_at IS NULL;
