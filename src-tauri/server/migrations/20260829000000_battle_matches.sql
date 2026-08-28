-- Испытания хранителя и записи партий.
--
-- Испытание задаёт ОБЕ стороны — и свою, и ту, которой играет гость. Это этюд,
-- а не поединок: расстановка составлена рукой, у неё есть решение, и её можно
-- переиграть сколько угодно раз.
--
-- Выбор осознанный и он про объём. Если бы гость приводил свою колоду, ему
-- понадобилось бы владение картами, владению — покупка, покупке — кошелёк с
-- церемонией; и первая партия отодвинулась бы на месяц. Собственные колоды
-- придут этапом позже и лягут рядом: `battle_decks` и второй способ начать матч.

CREATE TABLE IF NOT EXISTS battle_challenges (
    id           UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    slug         TEXT NOT NULL,
    title_en     TEXT NOT NULL CHECK (char_length(title_en) BETWEEN 1 AND 80),
    title_ru     TEXT NOT NULL CHECK (char_length(title_ru) BETWEEN 1 AND 80),
    note_en      TEXT CHECK (note_en IS NULL OR char_length(note_en) <= 400),
    note_ru      TEXT CHECK (note_ru IS NULL OR char_length(note_ru) <= 400),
    -- Расстановка по СЛАГАМ карт, а не по снимкам: испытание — это шаблон, и
    -- правка карты должна менять его вместе со всеми. Снимок делается в момент
    -- начала партии и ложится в `battle_matches`.
    -- {"playerBoard":[{"card":"boec","x":1,"y":4}], "playerHand":["shveya"],
    --  "keeperBoard":[…], "keeperHand":[…]}
    setup        TEXT NOT NULL CHECK (char_length(setup) <= 8000),
    -- Насколько глубоко думает хранитель. Сложность — это глубина перебора и
    -- только она: бот, которому дали лишнюю ману, ломает и честность, и всякую
    -- возможность измерить силу карты.
    bot_depth    SMALLINT NOT NULL DEFAULT 1 CHECK (bot_depth BETWEEN 1 AND 3),
    -- Пыль даётся ЗА ИСПЫТАНИЕ, а не за победу: переигрывать можно сколько
    -- угодно, заплатят однажды. Без этого PvE с наградой — это ферма.
    reward_dust  INTEGER NOT NULL DEFAULT 0 CHECK (reward_dust BETWEEN 0 AND 1000),
    status       TEXT NOT NULL DEFAULT 'draft'
                 CHECK (status IN ('draft', 'published', 'retired')),
    sort_order   INTEGER,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at   TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE UNIQUE INDEX IF NOT EXISTS battle_challenges_slug_idx ON battle_challenges (slug);
CREATE INDEX IF NOT EXISTS battle_challenges_shelf_idx
    ON battle_challenges (sort_order NULLS LAST, created_at DESC)
    WHERE status = 'published';

-- Партия.
--
-- Доска НЕ хранится: она свёртка журнала действий. Хранится снимок колод (чтобы
-- нерф не переписывал сыгранное), список действий (истина) и, отдельно, кэш
-- состояния — который в любой момент можно выбросить и пересчитать.
CREATE TABLE IF NOT EXISTS battle_matches (
    id            UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id       UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    challenge_id  UUID REFERENCES battle_challenges(id) ON DELETE SET NULL,
    -- Замороженные тела на момент начала. Ссылки бы означали, что правка карты
    -- переписывает историю всех сыгранных партий и пересмотр начинает врать.
    setup         TEXT NOT NULL,
    -- Версия правил карт на момент начала — чтобы старая партия пересматривалась
    -- по старым числам.
    rules_version INTEGER NOT NULL DEFAULT 1,
    -- Журнал действий. Истина о партии; всё остальное из него выводится.
    actions       TEXT NOT NULL DEFAULT '[]',
    -- Кэш состояния. Можно удалить — пересчитается свёрткой.
    board_cache   TEXT,
    -- Номер следующего действия. Повтор того же номера возвращает прежний ответ
    -- и ничего не пишет — тот же приём, что у покупки карты.
    seq           INTEGER NOT NULL DEFAULT 0,
    outcome       TEXT CHECK (outcome IN ('player', 'keeper', 'draw')),
    rounds        SMALLINT,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    finished_at   TIMESTAMPTZ
);

CREATE INDEX IF NOT EXISTS battle_matches_user_idx
    ON battle_matches (user_id, created_at DESC);
-- Одна незаконченная партия на испытание: вернувшийся гость продолжает свою, а
-- не заводит вторую тем же кликом.
CREATE UNIQUE INDEX IF NOT EXISTS battle_matches_open_idx
    ON battle_matches (user_id, challenge_id)
    WHERE outcome IS NULL;
