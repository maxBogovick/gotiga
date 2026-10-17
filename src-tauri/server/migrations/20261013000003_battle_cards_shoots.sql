-- Стреляет ли карта: лук, праща, самострел. Через строения (стены города) бьют
-- только стрелки с этой отметкой и маги (магический урон). Дальность 2 без
-- отметки — длинное оружие: по открытому полю бьёт, через стену нет
-- (BATTLE-GENRE-RESEARCH.md § 7.7).
ALTER TABLE battle_cards ADD COLUMN IF NOT EXISTS shoots BOOLEAN NOT NULL DEFAULT FALSE;
