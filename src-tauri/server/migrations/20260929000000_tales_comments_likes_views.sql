-- Отклик на байку: комментарии, лайк с дизлайком и просмотры.
--
-- Таблицы свои, а не общие с работами: у работы отклик — про вещь, которую
-- можно взять, у байки — про текст, который читают. Один общий столбец цели с
-- проверкой «заполнено ровно одно» пришлось бы держать во всех запросах
-- модерации разом, и первая же забытая проверка показала бы отклик о работе
-- под байкой.
--
-- Внешний ключ смотрит в gazette_leaves: байка физически лежит там строкой с
-- kind = 'tale'. Это место хранения, а не принадлежность — ни одна ручка и ни
-- один экран вестника этих таблиц не касается.

CREATE TABLE IF NOT EXISTS tale_comments (
    id           UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tale_id      UUID NOT NULL REFERENCES gazette_leaves(id) ON DELETE CASCADE,
    user_id      UUID REFERENCES users(id) ON DELETE SET NULL,
    author_name  TEXT NOT NULL,
    author_email TEXT,
    body         TEXT NOT NULL CHECK (char_length(body) >= 1 AND char_length(body) <= 1000),
    is_approved  BOOLEAN NOT NULL DEFAULT FALSE,
    admin_reply  TEXT,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_tale_comments_tale
    ON tale_comments (tale_id, is_approved, created_at);
CREATE INDEX IF NOT EXISTS idx_tale_comments_pending
    ON tale_comments (created_at) WHERE NOT is_approved;

-- Один голос на человека, и он же меняется на противоположный: value = +1 или
-- −1 в одной строке, а не две таблицы. Двумя таблицами «лайк» и «дизлайк»
-- одного читателя однажды оказались бы одновременно.
CREATE TABLE IF NOT EXISTS tale_likes (
    id            UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tale_id       UUID NOT NULL REFERENCES gazette_leaves(id) ON DELETE CASCADE,
    visitor_token VARCHAR(64) NOT NULL,
    user_id       UUID REFERENCES users(id) ON DELETE SET NULL,
    value         SMALLINT NOT NULL CHECK (value IN (-1, 1)),
    created_at    TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT tale_likes_visitor UNIQUE (tale_id, visitor_token)
);

CREATE UNIQUE INDEX IF NOT EXISTS tale_likes_user
    ON tale_likes (tale_id, user_id) WHERE user_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_tale_likes_tale ON tale_likes (tale_id);

-- Просмотр считается один на читателя в сутки: страницу открывают по ссылке,
-- возвращаются к ней и перезагружают, и число, растущее от F5, не сообщает
-- ничего. Время на странице и глубина прокрутки лежат не здесь, а в общем
-- потоке событий (page_view / page_engaged) — здесь только счётчик, который
-- страница показывает читателю.
CREATE TABLE IF NOT EXISTS tale_views (
    tale_id       UUID NOT NULL REFERENCES gazette_leaves(id) ON DELETE CASCADE,
    visitor_token VARCHAR(64) NOT NULL,
    seen_on       DATE NOT NULL DEFAULT CURRENT_DATE,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (tale_id, visitor_token, seen_on)
);

CREATE INDEX IF NOT EXISTS idx_tale_views_tale ON tale_views (tale_id);
