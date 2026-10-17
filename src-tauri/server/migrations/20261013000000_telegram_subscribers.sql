-- Книга дома, второй канал: Telegram.
--
-- Подтверждение через Telegram доказывает Telegram, а не почту, поэтому это
-- не способ подтвердить адрес из книги, а отдельная подписка: вести о новых
-- небылицах приходят в бота, почта не нужна. Подписчик подтверждён, когда
-- нажал «Запустить» в боте со своим одноразовым кодом: chat_id берётся из
-- самого сообщения Telegram, а не от страницы.
--
-- Пишет бот входа (как и записки о продолжениях): с ним человек сам начал
-- разговор.
CREATE TABLE IF NOT EXISTS telegram_subscribers (
    id                UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    -- Пусто, пока человек не нажал «Запустить»; тогда строка ещё не подписка.
    chat_id           BIGINT,
    username          TEXT,
    lang              TEXT NOT NULL DEFAULT 'en' CHECK (lang IN ('en', 'ru')),
    source            TEXT NOT NULL DEFAULT 'home',
    unsubscribe_token TEXT NOT NULL,
    -- Отпечаток кода из ссылки `?start=sub_…`; пуст у подтверждённых.
    confirm_hash      TEXT,
    confirm_sent_at   TIMESTAMPTZ,
    user_id           UUID REFERENCES users(id) ON DELETE SET NULL,
    ip                TEXT,
    created_at        TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    confirmed_at      TIMESTAMPTZ,
    unsubscribed_at   TIMESTAMPTZ,
    last_error        TEXT
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_telegram_subscribers_chat
    ON telegram_subscribers (chat_id) WHERE chat_id IS NOT NULL;
CREATE UNIQUE INDEX IF NOT EXISTS idx_telegram_subscribers_token
    ON telegram_subscribers (unsubscribe_token);
CREATE UNIQUE INDEX IF NOT EXISTS idx_telegram_subscribers_confirm
    ON telegram_subscribers (confirm_hash) WHERE confirm_hash IS NOT NULL;

-- Записки о новой небылице подписчикам-в-Telegram: та же очередь, что у писем
-- и записок о продолжениях (взятие, попытки, неделя срока).
CREATE TABLE IF NOT EXISTS tale_book_notes (
    tale_id       UUID NOT NULL REFERENCES gazette_leaves(id) ON DELETE CASCADE,
    subscriber_id UUID NOT NULL REFERENCES telegram_subscribers(id) ON DELETE CASCADE,
    lang          TEXT NOT NULL DEFAULT 'en' CHECK (lang IN ('en', 'ru')),
    created_at    TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    claimed_at    TIMESTAMPTZ,
    sent_at       TIMESTAMPTZ,
    attempts      INTEGER NOT NULL DEFAULT 0,
    last_error    TEXT,
    PRIMARY KEY (tale_id, subscriber_id)
);

CREATE INDEX IF NOT EXISTS idx_tale_book_notes_due
    ON tale_book_notes (created_at) WHERE sent_at IS NULL;
