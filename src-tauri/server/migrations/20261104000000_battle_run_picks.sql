-- Забег: журнал выбранных наград, чтобы игрок видел, что он уже получил
-- (до этого число бесплатных замен и вылеченные карты нигде не читались).
CREATE TABLE IF NOT EXISTS battle_run_picks (
    run_id    UUID NOT NULL REFERENCES battle_runs(id) ON DELETE CASCADE,
    -- Перед какой волной выбрана награда: по этому же номеру считается её ключ.
    wave_no   SMALLINT NOT NULL,
    kind      TEXT NOT NULL,
    amount    INTEGER NOT NULL DEFAULT 0,
    picked_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (run_id, wave_no)
);
