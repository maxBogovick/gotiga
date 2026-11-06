-- Забег: награда из трёх, замены, трофеи (`TASKS-BATTLE-RUN.md` § 3.2).

-- Одна открытая волна на забег: два быстрых нажатия «начать волну» не заводят вторую.
CREATE UNIQUE INDEX IF NOT EXISTS battle_matches_run_open_idx
    ON battle_matches (run_id) WHERE run_id IS NOT NULL AND outcome IS NULL;

ALTER TABLE battle_runs
    -- wave — можно начинать волну; reward — сначала выбрать награду.
    ADD COLUMN IF NOT EXISTS phase TEXT NOT NULL DEFAULT 'wave'
        CHECK (phase IN ('wave', 'reward')),
    -- До трёх наград на выбор (JSON), пока phase = 'reward'.
    ADD COLUMN IF NOT EXISTS reward_options TEXT,
    -- Бесплатные замены из награды.
    ADD COLUMN IF NOT EXISTS free_swaps INTEGER NOT NULL DEFAULT 0 CHECK (free_swaps >= 0);

-- Побеждённые враги забега: любую можно взять на полку за пыль.
-- Цена записывается в момент победы, по настройкам режима. Ноль — не продаётся.
CREATE TABLE IF NOT EXISTS battle_run_trophies (
    run_id     UUID NOT NULL REFERENCES battle_runs(id) ON DELETE CASCADE,
    card_id    UUID NOT NULL REFERENCES battle_cards(id) ON DELETE CASCADE,
    wave_no    SMALLINT NOT NULL,
    price_dust INTEGER NOT NULL CHECK (price_dust >= 0),
    taken      BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (run_id, card_id)
);
