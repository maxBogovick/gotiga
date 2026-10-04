-- Журнал объявлений в Telegram-канале: одна строка на то, что дом объявил или
-- решил не объявлять. Строка заводится, когда вещь впервые замечена на людях,
-- и уже не удаляется: скрыли и снова показали — второго поста не будет.
--
-- `skipped` — вещи, стоявшие на сайте до того, как канал подключили: без них
-- первый же тик вывалил бы в канал весь архив разом.
CREATE TABLE IF NOT EXISTS telegram_channel_posts (
    kind        TEXT        NOT NULL CHECK (kind IN ('work', 'leaf')),
    target_id   UUID        NOT NULL,
    seen_at     TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    skipped     BOOLEAN     NOT NULL DEFAULT FALSE,
    posted_at   TIMESTAMPTZ,
    message_id  BIGINT,
    attempts    INTEGER     NOT NULL DEFAULT 0,
    last_error  TEXT,
    PRIMARY KEY (kind, target_id)
);

CREATE INDEX IF NOT EXISTS idx_telegram_channel_posts_due
    ON telegram_channel_posts (seen_at)
    WHERE posted_at IS NULL AND NOT skipped;
