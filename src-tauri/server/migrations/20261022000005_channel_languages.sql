-- Каналы по языкам и пост о релизе.
--
-- Журнал объявлений теперь ведётся на каждый канал отдельно: одна и та же
-- работа выходит в английский, немецкий, испанский и итальянский каналы, и у
-- каждого своя строка (вышло ли, сколько попыток, чем кончилось). Прежние
-- строки принадлежат прежнему единственному каналу, который становится
-- английским.
ALTER TABLE telegram_channel_posts
    ADD COLUMN IF NOT EXISTS lang TEXT NOT NULL DEFAULT 'en'
        CHECK (lang IN ('en', 'ru', 'uk', 'es', 'de', 'it'));

ALTER TABLE telegram_channel_posts DROP CONSTRAINT IF EXISTS telegram_channel_posts_pkey;
ALTER TABLE telegram_channel_posts ADD PRIMARY KEY (kind, target_id, lang);

DROP INDEX IF EXISTS idx_telegram_channel_posts_due;
CREATE INDEX IF NOT EXISTS idx_telegram_channel_posts_due
    ON telegram_channel_posts (lang, seen_at)
    WHERE posted_at IS NULL AND NOT skipped;

-- Пост о релизе. Заметки лежат в файле `release-notes.md`, встроенном в
-- сервер; при первом запуске нового релиза сервер заводит здесь черновик, и
-- он ждёт, пока администратор не нажмёт «Отправить». Номер релиза (`id`) — из
-- файла заметок, поэтому повторная выкладка того же релиза или откат второго
-- черновика не заводят.
--
-- `since` — когда вышел предыдущий релиз: всё, что на сайте появилось позже,
-- попадёт в пост списком. Пусто у самого первого релиза: сравнивать не с чем.
-- `notes` — пункты по языкам, как их правит администратор:
-- {"en": {"new": [...], "better": [...]}, "de": {...}}.
CREATE TABLE IF NOT EXISTS release_posts (
    id          TEXT        PRIMARY KEY,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    since       TIMESTAMPTZ,
    notes       JSONB       NOT NULL DEFAULT '{}'::jsonb,
    status      TEXT        NOT NULL DEFAULT 'draft'
        CHECK (status IN ('draft', 'sent', 'dismissed'))
);

-- Что ушло в какой канал. Текст замораживается при отправке: список новых
-- работ считается в тот миг, и то, что написано здесь, — ровно то, что прочли
-- в канале.
CREATE TABLE IF NOT EXISTS release_post_sends (
    release_id  TEXT        NOT NULL REFERENCES release_posts (id) ON DELETE CASCADE,
    lang        TEXT        NOT NULL CHECK (lang IN ('en', 'ru', 'uk', 'es', 'de', 'it')),
    caption     TEXT        NOT NULL,
    posted_at   TIMESTAMPTZ,
    message_id  BIGINT,
    attempts    INTEGER     NOT NULL DEFAULT 0,
    last_error  TEXT,
    PRIMARY KEY (release_id, lang)
);
