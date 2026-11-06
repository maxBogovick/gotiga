-- Лечение одной карты наградой больше не применяется само: оно копится зарядом
-- (`heal_charges`), и игрок сам называет, кого лечить.
ALTER TABLE battle_runs
    ADD COLUMN IF NOT EXISTS heal_charges INTEGER NOT NULL DEFAULT 0 CHECK (heal_charges >= 0);
