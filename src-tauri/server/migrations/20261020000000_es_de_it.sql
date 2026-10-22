-- Spanish, German and Italian: three more languages beside English, Russian
-- and Ukrainian.
--
-- The same shape as 20261015000000_ukrainian.sql: every text that has a `_uk`
-- column gets `_es`, `_de` and `_it` with the same type and length limit.
-- Nothing is required in the new languages: `title_*` / `name_*` are
-- NOT NULL DEFAULT '' and the rest NULL, so every existing row stays valid. A
-- reader is shown their own language, then English, Russian, Ukrainian.

-- ── gazette_leaves ──
ALTER TABLE gazette_leaves
    ADD COLUMN IF NOT EXISTS title_es TEXT NOT NULL DEFAULT '',
    ADD COLUMN IF NOT EXISTS dek_es TEXT,
    ADD COLUMN IF NOT EXISTS body_es TEXT;
ALTER TABLE gazette_leaves
    ADD COLUMN IF NOT EXISTS title_de TEXT NOT NULL DEFAULT '',
    ADD COLUMN IF NOT EXISTS dek_de TEXT,
    ADD COLUMN IF NOT EXISTS body_de TEXT;
ALTER TABLE gazette_leaves
    ADD COLUMN IF NOT EXISTS title_it TEXT NOT NULL DEFAULT '',
    ADD COLUMN IF NOT EXISTS dek_it TEXT,
    ADD COLUMN IF NOT EXISTS body_it TEXT;
ALTER TABLE gazette_leaves DROP CONSTRAINT IF EXISTS gazette_leaves_title_es_check;
ALTER TABLE gazette_leaves ADD CONSTRAINT gazette_leaves_title_es_check
    CHECK (char_length(title_es) <= 200);
ALTER TABLE gazette_leaves DROP CONSTRAINT IF EXISTS gazette_leaves_title_de_check;
ALTER TABLE gazette_leaves ADD CONSTRAINT gazette_leaves_title_de_check
    CHECK (char_length(title_de) <= 200);
ALTER TABLE gazette_leaves DROP CONSTRAINT IF EXISTS gazette_leaves_title_it_check;
ALTER TABLE gazette_leaves ADD CONSTRAINT gazette_leaves_title_it_check
    CHECK (char_length(title_it) <= 200);
ALTER TABLE gazette_leaves DROP CONSTRAINT IF EXISTS gazette_leaves_dek_es_check;
ALTER TABLE gazette_leaves ADD CONSTRAINT gazette_leaves_dek_es_check
    CHECK (dek_es IS NULL OR char_length(dek_es) <= 500);
ALTER TABLE gazette_leaves DROP CONSTRAINT IF EXISTS gazette_leaves_dek_de_check;
ALTER TABLE gazette_leaves ADD CONSTRAINT gazette_leaves_dek_de_check
    CHECK (dek_de IS NULL OR char_length(dek_de) <= 500);
ALTER TABLE gazette_leaves DROP CONSTRAINT IF EXISTS gazette_leaves_dek_it_check;
ALTER TABLE gazette_leaves ADD CONSTRAINT gazette_leaves_dek_it_check
    CHECK (dek_it IS NULL OR char_length(dek_it) <= 500);
ALTER TABLE gazette_leaves DROP CONSTRAINT IF EXISTS gazette_leaves_body_es_check;
ALTER TABLE gazette_leaves ADD CONSTRAINT gazette_leaves_body_es_check
    CHECK (body_es IS NULL OR char_length(body_es) <= 12000);
ALTER TABLE gazette_leaves DROP CONSTRAINT IF EXISTS gazette_leaves_body_de_check;
ALTER TABLE gazette_leaves ADD CONSTRAINT gazette_leaves_body_de_check
    CHECK (body_de IS NULL OR char_length(body_de) <= 12000);
ALTER TABLE gazette_leaves DROP CONSTRAINT IF EXISTS gazette_leaves_body_it_check;
ALTER TABLE gazette_leaves ADD CONSTRAINT gazette_leaves_body_it_check
    CHECK (body_it IS NULL OR char_length(body_it) <= 12000);

-- ── battle_cards ──
ALTER TABLE battle_cards
    ADD COLUMN IF NOT EXISTS type_es TEXT,
    ADD COLUMN IF NOT EXISTS title_es TEXT NOT NULL DEFAULT '',
    ADD COLUMN IF NOT EXISTS effect_es TEXT,
    ADD COLUMN IF NOT EXISTS lore_es TEXT;
ALTER TABLE battle_cards
    ADD COLUMN IF NOT EXISTS type_de TEXT,
    ADD COLUMN IF NOT EXISTS title_de TEXT NOT NULL DEFAULT '',
    ADD COLUMN IF NOT EXISTS effect_de TEXT,
    ADD COLUMN IF NOT EXISTS lore_de TEXT;
ALTER TABLE battle_cards
    ADD COLUMN IF NOT EXISTS type_it TEXT,
    ADD COLUMN IF NOT EXISTS title_it TEXT NOT NULL DEFAULT '',
    ADD COLUMN IF NOT EXISTS effect_it TEXT,
    ADD COLUMN IF NOT EXISTS lore_it TEXT;
ALTER TABLE battle_cards DROP CONSTRAINT IF EXISTS battle_cards_type_es_check;
ALTER TABLE battle_cards ADD CONSTRAINT battle_cards_type_es_check
    CHECK (type_es IS NULL OR char_length(type_es) <= 40);
ALTER TABLE battle_cards DROP CONSTRAINT IF EXISTS battle_cards_type_de_check;
ALTER TABLE battle_cards ADD CONSTRAINT battle_cards_type_de_check
    CHECK (type_de IS NULL OR char_length(type_de) <= 40);
ALTER TABLE battle_cards DROP CONSTRAINT IF EXISTS battle_cards_type_it_check;
ALTER TABLE battle_cards ADD CONSTRAINT battle_cards_type_it_check
    CHECK (type_it IS NULL OR char_length(type_it) <= 40);
ALTER TABLE battle_cards DROP CONSTRAINT IF EXISTS battle_cards_title_es_check;
ALTER TABLE battle_cards ADD CONSTRAINT battle_cards_title_es_check
    CHECK (char_length(title_es) <= 80);
ALTER TABLE battle_cards DROP CONSTRAINT IF EXISTS battle_cards_title_de_check;
ALTER TABLE battle_cards ADD CONSTRAINT battle_cards_title_de_check
    CHECK (char_length(title_de) <= 80);
ALTER TABLE battle_cards DROP CONSTRAINT IF EXISTS battle_cards_title_it_check;
ALTER TABLE battle_cards ADD CONSTRAINT battle_cards_title_it_check
    CHECK (char_length(title_it) <= 80);
ALTER TABLE battle_cards DROP CONSTRAINT IF EXISTS battle_cards_effect_es_check;
ALTER TABLE battle_cards ADD CONSTRAINT battle_cards_effect_es_check
    CHECK (effect_es IS NULL OR char_length(effect_es) <= 400);
ALTER TABLE battle_cards DROP CONSTRAINT IF EXISTS battle_cards_effect_de_check;
ALTER TABLE battle_cards ADD CONSTRAINT battle_cards_effect_de_check
    CHECK (effect_de IS NULL OR char_length(effect_de) <= 400);
ALTER TABLE battle_cards DROP CONSTRAINT IF EXISTS battle_cards_effect_it_check;
ALTER TABLE battle_cards ADD CONSTRAINT battle_cards_effect_it_check
    CHECK (effect_it IS NULL OR char_length(effect_it) <= 400);
ALTER TABLE battle_cards DROP CONSTRAINT IF EXISTS battle_cards_lore_es_check;
ALTER TABLE battle_cards ADD CONSTRAINT battle_cards_lore_es_check
    CHECK (lore_es IS NULL OR char_length(lore_es) <= 400);
ALTER TABLE battle_cards DROP CONSTRAINT IF EXISTS battle_cards_lore_de_check;
ALTER TABLE battle_cards ADD CONSTRAINT battle_cards_lore_de_check
    CHECK (lore_de IS NULL OR char_length(lore_de) <= 400);
ALTER TABLE battle_cards DROP CONSTRAINT IF EXISTS battle_cards_lore_it_check;
ALTER TABLE battle_cards ADD CONSTRAINT battle_cards_lore_it_check
    CHECK (lore_it IS NULL OR char_length(lore_it) <= 400);

-- ── battle_races ──
ALTER TABLE battle_races
    ADD COLUMN IF NOT EXISTS name_es TEXT NOT NULL DEFAULT '',
    ADD COLUMN IF NOT EXISTS note_es TEXT;
ALTER TABLE battle_races
    ADD COLUMN IF NOT EXISTS name_de TEXT NOT NULL DEFAULT '',
    ADD COLUMN IF NOT EXISTS note_de TEXT;
ALTER TABLE battle_races
    ADD COLUMN IF NOT EXISTS name_it TEXT NOT NULL DEFAULT '',
    ADD COLUMN IF NOT EXISTS note_it TEXT;
ALTER TABLE battle_races DROP CONSTRAINT IF EXISTS battle_races_name_es_check;
ALTER TABLE battle_races ADD CONSTRAINT battle_races_name_es_check
    CHECK (char_length(name_es) <= 60);
ALTER TABLE battle_races DROP CONSTRAINT IF EXISTS battle_races_name_de_check;
ALTER TABLE battle_races ADD CONSTRAINT battle_races_name_de_check
    CHECK (char_length(name_de) <= 60);
ALTER TABLE battle_races DROP CONSTRAINT IF EXISTS battle_races_name_it_check;
ALTER TABLE battle_races ADD CONSTRAINT battle_races_name_it_check
    CHECK (char_length(name_it) <= 60);
ALTER TABLE battle_races DROP CONSTRAINT IF EXISTS battle_races_note_es_check;
ALTER TABLE battle_races ADD CONSTRAINT battle_races_note_es_check
    CHECK (note_es IS NULL OR char_length(note_es) <= 200);
ALTER TABLE battle_races DROP CONSTRAINT IF EXISTS battle_races_note_de_check;
ALTER TABLE battle_races ADD CONSTRAINT battle_races_note_de_check
    CHECK (note_de IS NULL OR char_length(note_de) <= 200);
ALTER TABLE battle_races DROP CONSTRAINT IF EXISTS battle_races_note_it_check;
ALTER TABLE battle_races ADD CONSTRAINT battle_races_note_it_check
    CHECK (note_it IS NULL OR char_length(note_it) <= 200);

-- ── battle_keywords ──
ALTER TABLE battle_keywords
    ADD COLUMN IF NOT EXISTS name_es TEXT NOT NULL DEFAULT '',
    ADD COLUMN IF NOT EXISTS rules_es TEXT;
ALTER TABLE battle_keywords
    ADD COLUMN IF NOT EXISTS name_de TEXT NOT NULL DEFAULT '',
    ADD COLUMN IF NOT EXISTS rules_de TEXT;
ALTER TABLE battle_keywords
    ADD COLUMN IF NOT EXISTS name_it TEXT NOT NULL DEFAULT '',
    ADD COLUMN IF NOT EXISTS rules_it TEXT;
ALTER TABLE battle_keywords DROP CONSTRAINT IF EXISTS battle_keywords_name_es_check;
ALTER TABLE battle_keywords ADD CONSTRAINT battle_keywords_name_es_check
    CHECK (char_length(name_es) <= 60);
ALTER TABLE battle_keywords DROP CONSTRAINT IF EXISTS battle_keywords_name_de_check;
ALTER TABLE battle_keywords ADD CONSTRAINT battle_keywords_name_de_check
    CHECK (char_length(name_de) <= 60);
ALTER TABLE battle_keywords DROP CONSTRAINT IF EXISTS battle_keywords_name_it_check;
ALTER TABLE battle_keywords ADD CONSTRAINT battle_keywords_name_it_check
    CHECK (char_length(name_it) <= 60);
ALTER TABLE battle_keywords DROP CONSTRAINT IF EXISTS battle_keywords_rules_es_check;
ALTER TABLE battle_keywords ADD CONSTRAINT battle_keywords_rules_es_check
    CHECK (rules_es IS NULL OR char_length(rules_es) <= 300);
ALTER TABLE battle_keywords DROP CONSTRAINT IF EXISTS battle_keywords_rules_de_check;
ALTER TABLE battle_keywords ADD CONSTRAINT battle_keywords_rules_de_check
    CHECK (rules_de IS NULL OR char_length(rules_de) <= 300);
ALTER TABLE battle_keywords DROP CONSTRAINT IF EXISTS battle_keywords_rules_it_check;
ALTER TABLE battle_keywords ADD CONSTRAINT battle_keywords_rules_it_check
    CHECK (rules_it IS NULL OR char_length(rules_it) <= 300);

-- ── battle_challenges ──
ALTER TABLE battle_challenges
    ADD COLUMN IF NOT EXISTS title_es TEXT NOT NULL DEFAULT '',
    ADD COLUMN IF NOT EXISTS note_es TEXT;
ALTER TABLE battle_challenges
    ADD COLUMN IF NOT EXISTS title_de TEXT NOT NULL DEFAULT '',
    ADD COLUMN IF NOT EXISTS note_de TEXT;
ALTER TABLE battle_challenges
    ADD COLUMN IF NOT EXISTS title_it TEXT NOT NULL DEFAULT '',
    ADD COLUMN IF NOT EXISTS note_it TEXT;
ALTER TABLE battle_challenges DROP CONSTRAINT IF EXISTS battle_challenges_title_es_check;
ALTER TABLE battle_challenges ADD CONSTRAINT battle_challenges_title_es_check
    CHECK (char_length(title_es) <= 80);
ALTER TABLE battle_challenges DROP CONSTRAINT IF EXISTS battle_challenges_title_de_check;
ALTER TABLE battle_challenges ADD CONSTRAINT battle_challenges_title_de_check
    CHECK (char_length(title_de) <= 80);
ALTER TABLE battle_challenges DROP CONSTRAINT IF EXISTS battle_challenges_title_it_check;
ALTER TABLE battle_challenges ADD CONSTRAINT battle_challenges_title_it_check
    CHECK (char_length(title_it) <= 80);
ALTER TABLE battle_challenges DROP CONSTRAINT IF EXISTS battle_challenges_note_es_check;
ALTER TABLE battle_challenges ADD CONSTRAINT battle_challenges_note_es_check
    CHECK (note_es IS NULL OR char_length(note_es) <= 400);
ALTER TABLE battle_challenges DROP CONSTRAINT IF EXISTS battle_challenges_note_de_check;
ALTER TABLE battle_challenges ADD CONSTRAINT battle_challenges_note_de_check
    CHECK (note_de IS NULL OR char_length(note_de) <= 400);
ALTER TABLE battle_challenges DROP CONSTRAINT IF EXISTS battle_challenges_note_it_check;
ALTER TABLE battle_challenges ADD CONSTRAINT battle_challenges_note_it_check
    CHECK (note_it IS NULL OR char_length(note_it) <= 400);

-- ── battle_errands ──
ALTER TABLE battle_errands
    ADD COLUMN IF NOT EXISTS title_es TEXT NOT NULL DEFAULT '',
    ADD COLUMN IF NOT EXISTS note_es TEXT;
ALTER TABLE battle_errands
    ADD COLUMN IF NOT EXISTS title_de TEXT NOT NULL DEFAULT '',
    ADD COLUMN IF NOT EXISTS note_de TEXT;
ALTER TABLE battle_errands
    ADD COLUMN IF NOT EXISTS title_it TEXT NOT NULL DEFAULT '',
    ADD COLUMN IF NOT EXISTS note_it TEXT;
ALTER TABLE battle_errands DROP CONSTRAINT IF EXISTS battle_errands_title_es_check;
ALTER TABLE battle_errands ADD CONSTRAINT battle_errands_title_es_check
    CHECK (char_length(title_es) <= 120);
ALTER TABLE battle_errands DROP CONSTRAINT IF EXISTS battle_errands_title_de_check;
ALTER TABLE battle_errands ADD CONSTRAINT battle_errands_title_de_check
    CHECK (char_length(title_de) <= 120);
ALTER TABLE battle_errands DROP CONSTRAINT IF EXISTS battle_errands_title_it_check;
ALTER TABLE battle_errands ADD CONSTRAINT battle_errands_title_it_check
    CHECK (char_length(title_it) <= 120);
ALTER TABLE battle_errands DROP CONSTRAINT IF EXISTS battle_errands_note_es_check;
ALTER TABLE battle_errands ADD CONSTRAINT battle_errands_note_es_check
    CHECK (note_es IS NULL OR char_length(note_es) <= 400);
ALTER TABLE battle_errands DROP CONSTRAINT IF EXISTS battle_errands_note_de_check;
ALTER TABLE battle_errands ADD CONSTRAINT battle_errands_note_de_check
    CHECK (note_de IS NULL OR char_length(note_de) <= 400);
ALTER TABLE battle_errands DROP CONSTRAINT IF EXISTS battle_errands_note_it_check;
ALTER TABLE battle_errands ADD CONSTRAINT battle_errands_note_it_check
    CHECK (note_it IS NULL OR char_length(note_it) <= 400);

-- ── studio_races ──
ALTER TABLE studio_races
    ADD COLUMN IF NOT EXISTS name_es TEXT NOT NULL DEFAULT '',
    ADD COLUMN IF NOT EXISTS note_es TEXT;
ALTER TABLE studio_races
    ADD COLUMN IF NOT EXISTS name_de TEXT NOT NULL DEFAULT '',
    ADD COLUMN IF NOT EXISTS note_de TEXT;
ALTER TABLE studio_races
    ADD COLUMN IF NOT EXISTS name_it TEXT NOT NULL DEFAULT '',
    ADD COLUMN IF NOT EXISTS note_it TEXT;
ALTER TABLE studio_races DROP CONSTRAINT IF EXISTS studio_races_name_es_check;
ALTER TABLE studio_races ADD CONSTRAINT studio_races_name_es_check
    CHECK (char_length(name_es) <= 60);
ALTER TABLE studio_races DROP CONSTRAINT IF EXISTS studio_races_name_de_check;
ALTER TABLE studio_races ADD CONSTRAINT studio_races_name_de_check
    CHECK (char_length(name_de) <= 60);
ALTER TABLE studio_races DROP CONSTRAINT IF EXISTS studio_races_name_it_check;
ALTER TABLE studio_races ADD CONSTRAINT studio_races_name_it_check
    CHECK (char_length(name_it) <= 60);
ALTER TABLE studio_races DROP CONSTRAINT IF EXISTS studio_races_note_es_check;
ALTER TABLE studio_races ADD CONSTRAINT studio_races_note_es_check
    CHECK (note_es IS NULL OR char_length(note_es) <= 200);
ALTER TABLE studio_races DROP CONSTRAINT IF EXISTS studio_races_note_de_check;
ALTER TABLE studio_races ADD CONSTRAINT studio_races_note_de_check
    CHECK (note_de IS NULL OR char_length(note_de) <= 200);
ALTER TABLE studio_races DROP CONSTRAINT IF EXISTS studio_races_note_it_check;
ALTER TABLE studio_races ADD CONSTRAINT studio_races_note_it_check
    CHECK (note_it IS NULL OR char_length(note_it) <= 200);

-- ── The language a person worked or asked in: now one of six ──
ALTER TABLE tale_sequel_wishes DROP CONSTRAINT IF EXISTS tale_sequel_wishes_lang_check;
ALTER TABLE tale_sequel_wishes ADD CONSTRAINT tale_sequel_wishes_lang_check
    CHECK (lang IN ('en', 'ru', 'uk', 'es', 'de', 'it'));
ALTER TABLE tale_poll_votes DROP CONSTRAINT IF EXISTS tale_poll_votes_lang_check;
ALTER TABLE tale_poll_votes ADD CONSTRAINT tale_poll_votes_lang_check
    CHECK (lang IN ('en', 'ru', 'uk', 'es', 'de', 'it'));
ALTER TABLE tale_letters DROP CONSTRAINT IF EXISTS tale_letters_lang_check;
ALTER TABLE tale_letters ADD CONSTRAINT tale_letters_lang_check
    CHECK (lang IN ('en', 'ru', 'uk', 'es', 'de', 'it'));
ALTER TABLE tale_notes DROP CONSTRAINT IF EXISTS tale_notes_lang_check;
ALTER TABLE tale_notes ADD CONSTRAINT tale_notes_lang_check
    CHECK (lang IN ('en', 'ru', 'uk', 'es', 'de', 'it'));
ALTER TABLE tale_book_notes DROP CONSTRAINT IF EXISTS tale_book_notes_lang_check;
ALTER TABLE tale_book_notes ADD CONSTRAINT tale_book_notes_lang_check
    CHECK (lang IN ('en', 'ru', 'uk', 'es', 'de', 'it'));
ALTER TABLE telegram_subscribers DROP CONSTRAINT IF EXISTS telegram_subscribers_lang_check;
ALTER TABLE telegram_subscribers ADD CONSTRAINT telegram_subscribers_lang_check
    CHECK (lang IN ('en', 'ru', 'uk', 'es', 'de', 'it'));
ALTER TABLE studio_races DROP CONSTRAINT IF EXISTS studio_races_lang_check;
ALTER TABLE studio_races ADD CONSTRAINT studio_races_lang_check
    CHECK (lang IN ('en', 'ru', 'uk', 'es', 'de', 'it'));
ALTER TABLE studio_cards DROP CONSTRAINT IF EXISTS studio_cards_lang_check;
ALTER TABLE studio_cards ADD CONSTRAINT studio_cards_lang_check
    CHECK (lang IN ('en', 'ru', 'uk', 'es', 'de', 'it'));
ALTER TABLE studio_motions DROP CONSTRAINT IF EXISTS studio_motions_lang_check;
ALTER TABLE studio_motions ADD CONSTRAINT studio_motions_lang_check
    CHECK (lang IN ('en', 'ru', 'uk', 'es', 'de', 'it'));

-- Texts in the new languages (errands, siege gear, tales, cards, works) come
-- from 20261020000002 / 20261020000003, generated by tools/i18n/make_migration.py
-- from tools/i18n/content/: one source for every translated text.
