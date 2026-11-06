-- Маршрут дня: один маршрут и одно зерно на всех на сутки (UTC). Один забег дня
-- на человека; остальные забеги `daily_date` не имеют.
ALTER TABLE battle_runs ADD COLUMN IF NOT EXISTS daily_date DATE;

CREATE UNIQUE INDEX IF NOT EXISTS battle_runs_daily_idx
    ON battle_runs (user_id, daily_date) WHERE daily_date IS NOT NULL;
CREATE INDEX IF NOT EXISTS battle_runs_daily_day_idx
    ON battle_runs (daily_date) WHERE daily_date IS NOT NULL;
