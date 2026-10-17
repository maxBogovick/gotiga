-- Урон по расам: «+N к урону по картам отмеченных рас» и «−N урона от карт
-- отмеченных рас». JSON `{"attack","against":[id…],"defence","from":[id…]}`,
-- тем же обычаем, что `level_frames` и `motion_wear`. NULL — раса ни против
-- кого не настроена. Ссылки на расы — id, а не слаг: переименование расы не
-- должно молча обнулять таблицу.
ALTER TABLE battle_races ADD COLUMN IF NOT EXISTS race_damage TEXT;
ALTER TABLE battle_races DROP CONSTRAINT IF EXISTS battle_races_race_damage_check;
ALTER TABLE battle_races ADD CONSTRAINT battle_races_race_damage_check
    CHECK (race_damage IS NULL OR char_length(race_damage) <= 4000);
