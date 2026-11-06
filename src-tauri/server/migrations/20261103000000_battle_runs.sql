-- Забег: маршрут из волн и забеги игроков (`TASKS-BATTLE-RUN.md`).
--
-- Маршрут — цепочка волн. Волна опирается на этюд (`battle_challenges`): оттуда
-- берутся поле, местность, правила и враги, стоящие с начала. Сверх этюда у
-- волны есть расписание подкрепления, условие победы и число карт, которое
-- игрок выводит. Движок ничего не знает о маршрутах: он получает расстановку.

CREATE TABLE IF NOT EXISTS battle_routes (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    slug        TEXT NOT NULL,
    title_en    TEXT NOT NULL CHECK (char_length(title_en) BETWEEN 1 AND 80),
    title_ru    TEXT NOT NULL CHECK (char_length(title_ru) BETWEEN 1 AND 80),
    title_uk    TEXT NOT NULL DEFAULT '' CHECK (char_length(title_uk) <= 80),
    title_es    TEXT NOT NULL DEFAULT '' CHECK (char_length(title_es) <= 80),
    title_de    TEXT NOT NULL DEFAULT '' CHECK (char_length(title_de) <= 80),
    title_it    TEXT NOT NULL DEFAULT '' CHECK (char_length(title_it) <= 80),
    -- Сколько волн проходит гость. NULL — гостю маршрут не открыт.
    guest_waves SMALLINT CHECK (guest_waves IS NULL OR guest_waves BETWEEN 1 AND 6),
    status      TEXT NOT NULL DEFAULT 'draft'
                CHECK (status IN ('draft', 'published', 'retired')),
    sort_order  INTEGER,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE UNIQUE INDEX IF NOT EXISTS battle_routes_slug_idx ON battle_routes (slug);

CREATE TABLE IF NOT EXISTS battle_route_waves (
    id           UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    route_id     UUID NOT NULL REFERENCES battle_routes(id) ON DELETE CASCADE,
    wave_no      SMALLINT NOT NULL CHECK (wave_no BETWEEN 1 AND 12),
    -- Этюд-основа: поле, местность, правила, враги с начала, глубина руки.
    -- Этюд нельзя удалить, пока на нём стоит волна.
    challenge_id UUID NOT NULL REFERENCES battle_challenges(id) ON DELETE RESTRICT,
    -- Подкрепление: [{"round":2,"x":1,"y":0,"card":"slug"}]
    reinforcements TEXT NOT NULL DEFAULT '[]' CHECK (char_length(reinforcements) <= 4000),
    -- Условие победы — закрытый список движка (`WaveGoal`), как JSON.
    -- NULL — обычные правила партии.
    goal         TEXT CHECK (goal IS NULL OR char_length(goal) <= 400),
    min_cards    SMALLINT NOT NULL DEFAULT 1 CHECK (min_cards BETWEEN 1 AND 6),
    max_cards    SMALLINT NOT NULL DEFAULT 3 CHECK (max_cards BETWEEN 1 AND 6),
    -- Пыль за волну; выдаётся шагом награды (сервер, не движок).
    reward_dust  INTEGER NOT NULL DEFAULT 0 CHECK (reward_dust BETWEEN 0 AND 1000),
    -- Раса волны для показа перед боем (слаг расы) — подсказка, не правило.
    race_hint    TEXT,
    CONSTRAINT battle_route_waves_cards_order CHECK (min_cards <= max_cards),
    CONSTRAINT battle_route_waves_unique UNIQUE (route_id, wave_no)
);

-- Забег игрока или гостя. Сохраняется после каждой волны.
CREATE TABLE IF NOT EXISTS battle_runs (
    id            UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id       UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    route_id      UUID NOT NULL REFERENCES battle_routes(id) ON DELETE RESTRICT,
    -- Волна, которую предстоит или идёт играть (с единицы).
    wave_no       SMALLINT NOT NULL DEFAULT 1 CHECK (wave_no >= 1),
    -- Сколько волн в этом забеге: у гостя — пробные, у остальных — весь маршрут.
    waves_total   SMALLINT NOT NULL CHECK (waves_total >= 1),
    state         TEXT NOT NULL DEFAULT 'active' CHECK (state IN ('active', 'lost', 'won')),
    -- Отряд: [{"cardId":"…","health":7,"dead":false,"loaned":false}].
    -- health null — полное. loaned — карта выдана забегом, а не из собрания.
    squad         TEXT NOT NULL DEFAULT '[]',
    -- Сколько замен уже сделано (цена растёт с каждой; шаг 5).
    swaps         INTEGER NOT NULL DEFAULT 0 CHECK (swaps >= 0),
    -- Зерно случайности забега. Считает сервер; повтор запроса не меняет итог.
    seed          BIGINT NOT NULL,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at    TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    finished_at   TIMESTAMPTZ
);

-- Один незаконченный забег на человека: вернувшийся продолжает свой.
CREATE UNIQUE INDEX IF NOT EXISTS battle_runs_active_idx
    ON battle_runs (user_id) WHERE state = 'active';
CREATE INDEX IF NOT EXISTS battle_runs_user_idx ON battle_runs (user_id, created_at DESC);

-- Партия-волна: обычная партия, помеченная забегом. challenge_id остаётся
-- пустым — награда и рекорд этюда на волны не распространяются.
ALTER TABLE battle_matches
    ADD COLUMN IF NOT EXISTS run_id   UUID REFERENCES battle_runs(id) ON DELETE SET NULL,
    ADD COLUMN IF NOT EXISTS run_wave SMALLINT,
    -- Кто из отряда в каком месте списка `player_board` (id карты по порядку):
    -- по нему здоровье после волны возвращается в отряд.
    ADD COLUMN IF NOT EXISTS run_cards TEXT;

CREATE INDEX IF NOT EXISTS battle_matches_run_idx ON battle_matches (run_id) WHERE run_id IS NOT NULL;
