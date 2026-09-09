-- Сезоны: неделя, оценки, вердикт.
--
-- Устройство недели: понедельник открывает приём и оценки, суббота закрывает,
-- в ночь на воскресенье считается вердикт и тройка уходит хозяину. Считает
-- фоновая задача, и она обязана быть ИДЕМПОТЕНТНОЙ: перезапуск сервера в
-- воскресенье не должен подвести вердикт дважды и заплатить дважды. Отсюда
-- `state`: сезон в состоянии `judged` больше не считается никогда.

CREATE TABLE IF NOT EXISTS studio_seasons (
    id         UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    -- Порядковый номер. Им сезон и зовут: «сезон 14» — это адрес страницы и
    -- слово, которым о нём говорят.
    number     INTEGER NOT NULL,
    opens_at   TIMESTAMPTZ NOT NULL,
    closes_at  TIMESTAMPTZ NOT NULL,
    verdict_at TIMESTAMPTZ NOT NULL,
    state      TEXT NOT NULL CHECK (state IN ('open', 'closed', 'judged')) DEFAULT 'open',
    -- Тему задаёт хозяин. Это ПРИГЛАШЕНИЕ, а не проверка: работа мимо темы
    -- участвует наравне, и никакая машина её не отсеивает. Пустая тема —
    -- свободный сезон, обычное состояние, а не поломка.
    theme      TEXT CHECK (theme IS NULL OR char_length(theme) <= 120),
    theme_note TEXT CHECK (theme_note IS NULL OR char_length(theme_note) <= 400),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE UNIQUE INDEX IF NOT EXISTS studio_seasons_number_idx ON studio_seasons (number);
CREATE INDEX IF NOT EXISTS studio_seasons_open_idx ON studio_seasons (opens_at DESC);

-- Заявка рамы в сезон.
--
-- `UNIQUE (season_id, author_id)` — одна заявка от человека в сезон, и это
-- правило схемы, а не проверки в службе: автор с пятью рамами не должен
-- занимать собой всю полку.
CREATE TABLE IF NOT EXISTS studio_entries (
    id         UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    season_id  UUID NOT NULL REFERENCES studio_seasons(id) ON DELETE CASCADE,
    frame_id   UUID NOT NULL REFERENCES studio_frames(id) ON DELETE CASCADE,
    author_id  UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    entered_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    -- Итог считается вердиктом и остаётся навсегда: страница прошедшего сезона
    -- не должна пересчитываться заново по нынешним правилам.
    score      NUMERIC,
    votes      INTEGER NOT NULL DEFAULT 0,
    weight     NUMERIC NOT NULL DEFAULT 0,
    place      SMALLINT,
    to_keeper  BOOLEAN NOT NULL DEFAULT false,
    CONSTRAINT studio_entries_one_per_author UNIQUE (season_id, author_id),
    CONSTRAINT studio_entries_one_per_frame UNIQUE (season_id, frame_id)
);

CREATE INDEX IF NOT EXISTS studio_entries_season_idx ON studio_entries (season_id, place NULLS LAST);
CREATE INDEX IF NOT EXISTS studio_entries_keeper_idx ON studio_entries (season_id) WHERE to_keeper;

-- Оценка. От одного до пяти.
--
-- `weight` пишется В МОМЕНТ ГОЛОСА и больше не меняется. Человек мог за неделю
-- купить карту или сыграть партию, и пересчёт веса при вердикте задним числом
-- изменил бы уже отданные голоса — то есть переписал бы прошлое.
--
-- `UNIQUE (entry_id, voter_id)`: переоценить можно (мнение меняется), удвоить
-- нельзя.
CREATE TABLE IF NOT EXISTS studio_ratings (
    entry_id   UUID NOT NULL REFERENCES studio_entries(id) ON DELETE CASCADE,
    voter_id   UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    value      SMALLINT NOT NULL CHECK (value BETWEEN 1 AND 5),
    weight     NUMERIC NOT NULL CHECK (weight > 0),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (entry_id, voter_id)
);

CREATE INDEX IF NOT EXISTS studio_ratings_voter_idx ON studio_ratings (voter_id, created_at);

-- Жалоба. Заводится вместе с галереей, а не после первого случая: чужой арт и
-- непристойное приходят на второй день, а не на второй год.
CREATE TABLE IF NOT EXISTS studio_reports (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    frame_id    UUID NOT NULL REFERENCES studio_frames(id) ON DELETE CASCADE,
    reporter_id UUID REFERENCES users(id) ON DELETE SET NULL,
    reason      TEXT NOT NULL CHECK (char_length(reason) <= 40),
    note        TEXT CHECK (note IS NULL OR char_length(note) <= 600),
    state       TEXT NOT NULL CHECK (state IN ('open', 'closed')) DEFAULT 'open',
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS studio_reports_open_idx ON studio_reports (created_at) WHERE state = 'open';

-- Допуск: хозяин смотрит работу ДО того, как её увидят люди.
--
-- Не то же, что утверждение. Допуск — «этому можно на люди», несколько минут в
-- неделю. Утверждение — «этому быть в игре и в продаже», и оно после вердикта.
ALTER TABLE studio_frames ADD COLUMN IF NOT EXISTS admitted_at TIMESTAMPTZ;
ALTER TABLE studio_frames ADD COLUMN IF NOT EXISTS admit_word TEXT;
ALTER TABLE studio_frames DROP CONSTRAINT IF EXISTS studio_frames_admit_word_check;
ALTER TABLE studio_frames ADD CONSTRAINT studio_frames_admit_word_check
    CHECK (admit_word IS NULL OR char_length(admit_word) <= 600);
-- Сколько раз рама выставлялась. Потолок — два: доработать и попробовать ещё
-- раз можно, ходить с одной рамой по сезонам бесконечно — нет.
ALTER TABLE studio_frames ADD COLUMN IF NOT EXISTS entries_used SMALLINT NOT NULL DEFAULT 0;

CREATE INDEX IF NOT EXISTS studio_frames_admission_idx
    ON studio_frames (updated_at) WHERE status = 'published' AND admitted_at IS NULL;
