-- A draft card may be named in one language.
--
-- The save stopped copying one title into the other (the English shelf then
-- printed Cyrillic as if it were English), and a published card without both
-- titles is refused by `prose_blockers` ("noTitle"). The table still demanded
-- both from the first day, so every one-language draft failed with a 500 on
-- `battle_cards_title_ru_check` instead of being saved.
--
-- Each title may now be empty; at least one of the two may not.
ALTER TABLE battle_cards DROP CONSTRAINT IF EXISTS battle_cards_title_en_check;
ALTER TABLE battle_cards DROP CONSTRAINT IF EXISTS battle_cards_title_ru_check;

ALTER TABLE battle_cards ADD CONSTRAINT battle_cards_title_en_check
    CHECK (char_length(title_en) <= 80);
ALTER TABLE battle_cards ADD CONSTRAINT battle_cards_title_ru_check
    CHECK (char_length(title_ru) <= 80);
ALTER TABLE battle_cards ADD CONSTRAINT battle_cards_has_title
    CHECK (char_length(title_en) > 0 OR char_length(title_ru) > 0);
