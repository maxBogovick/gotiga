-- A dressed card is written to be read.
--
-- The effect started at one line, which is right for a bare card held in a
-- hand. A card wearing a photographed frame has a panel under the picture, and
-- 160 characters could not hold three named abilities — the very thing such a
-- card exists to show. The shelf still clamps what will not fit; the table no
-- longer refuses it.
--
-- The original CHECKs were written inline, so Postgres named them itself.
-- Dropping by that generated name is safe here (the table is one migration old
-- and nothing has renamed it), but IF EXISTS keeps a hand-edited database from
-- failing the whole migration over a constraint that is already gone.
ALTER TABLE battle_cards DROP CONSTRAINT IF EXISTS battle_cards_effect_en_check;
ALTER TABLE battle_cards DROP CONSTRAINT IF EXISTS battle_cards_effect_ru_check;

ALTER TABLE battle_cards ADD CONSTRAINT battle_cards_effect_en_check
    CHECK (effect_en IS NULL OR char_length(effect_en) <= 400);
ALTER TABLE battle_cards ADD CONSTRAINT battle_cards_effect_ru_check
    CHECK (effect_ru IS NULL OR char_length(effect_ru) <= 400);
