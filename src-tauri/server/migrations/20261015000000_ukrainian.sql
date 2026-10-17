-- Ukrainian: a third language next to English and Russian.
--
-- Every bilingual text gets a `_uk` column beside its `_ru` one, with the same
-- length limit. Nothing is required in Ukrainian: the `title_*` columns are
-- NOT NULL DEFAULT '' (and, unlike English and Russian, may stay empty) so
-- that every existing row stays valid and the keeper can translate at her own
-- pace. A reader of Ukrainian is shown English where Ukrainian is missing
-- (see `firstText` on the client), never a blank.

-- ── Gazette (and with it the tales: a tale is a gazette leaf) ───────────────
ALTER TABLE gazette_leaves
    ADD COLUMN IF NOT EXISTS title_uk TEXT NOT NULL DEFAULT '',
    ADD COLUMN IF NOT EXISTS dek_uk   TEXT,
    ADD COLUMN IF NOT EXISTS body_uk  TEXT;
ALTER TABLE gazette_leaves DROP CONSTRAINT IF EXISTS gazette_leaves_title_uk_check;
ALTER TABLE gazette_leaves ADD CONSTRAINT gazette_leaves_title_uk_check
    CHECK (char_length(title_uk) <= 200);
ALTER TABLE gazette_leaves DROP CONSTRAINT IF EXISTS gazette_leaves_dek_uk_check;
ALTER TABLE gazette_leaves ADD CONSTRAINT gazette_leaves_dek_uk_check
    CHECK (dek_uk IS NULL OR char_length(dek_uk) <= 500);
ALTER TABLE gazette_leaves DROP CONSTRAINT IF EXISTS gazette_leaves_body_uk_check;
ALTER TABLE gazette_leaves ADD CONSTRAINT gazette_leaves_body_uk_check
    CHECK (body_uk IS NULL OR char_length(body_uk) <= 12000);

-- ── Battle cards ────────────────────────────────────────────────────────────
ALTER TABLE battle_cards
    ADD COLUMN IF NOT EXISTS type_uk   TEXT,
    ADD COLUMN IF NOT EXISTS title_uk  TEXT NOT NULL DEFAULT '',
    ADD COLUMN IF NOT EXISTS effect_uk TEXT,
    ADD COLUMN IF NOT EXISTS lore_uk   TEXT;
ALTER TABLE battle_cards DROP CONSTRAINT IF EXISTS battle_cards_type_uk_check;
ALTER TABLE battle_cards ADD CONSTRAINT battle_cards_type_uk_check
    CHECK (type_uk IS NULL OR char_length(type_uk) <= 40);
ALTER TABLE battle_cards DROP CONSTRAINT IF EXISTS battle_cards_title_uk_check;
ALTER TABLE battle_cards ADD CONSTRAINT battle_cards_title_uk_check
    CHECK (char_length(title_uk) <= 80);
ALTER TABLE battle_cards DROP CONSTRAINT IF EXISTS battle_cards_effect_uk_check;
ALTER TABLE battle_cards ADD CONSTRAINT battle_cards_effect_uk_check
    CHECK (effect_uk IS NULL OR char_length(effect_uk) <= 400);
ALTER TABLE battle_cards DROP CONSTRAINT IF EXISTS battle_cards_lore_uk_check;
ALTER TABLE battle_cards ADD CONSTRAINT battle_cards_lore_uk_check
    CHECK (lore_uk IS NULL OR char_length(lore_uk) <= 400);

-- ── Races, keywords ─────────────────────────────────────────────────────────
ALTER TABLE battle_races
    ADD COLUMN IF NOT EXISTS name_uk TEXT NOT NULL DEFAULT '',
    ADD COLUMN IF NOT EXISTS note_uk TEXT;
ALTER TABLE battle_races DROP CONSTRAINT IF EXISTS battle_races_name_uk_check;
ALTER TABLE battle_races ADD CONSTRAINT battle_races_name_uk_check
    CHECK (char_length(name_uk) <= 60);
ALTER TABLE battle_races DROP CONSTRAINT IF EXISTS battle_races_note_uk_check;
ALTER TABLE battle_races ADD CONSTRAINT battle_races_note_uk_check
    CHECK (note_uk IS NULL OR char_length(note_uk) <= 200);

ALTER TABLE battle_keywords
    ADD COLUMN IF NOT EXISTS name_uk  TEXT NOT NULL DEFAULT '',
    ADD COLUMN IF NOT EXISTS rules_uk TEXT;
ALTER TABLE battle_keywords DROP CONSTRAINT IF EXISTS battle_keywords_name_uk_check;
ALTER TABLE battle_keywords ADD CONSTRAINT battle_keywords_name_uk_check
    CHECK (char_length(name_uk) <= 60);
ALTER TABLE battle_keywords DROP CONSTRAINT IF EXISTS battle_keywords_rules_uk_check;
ALTER TABLE battle_keywords ADD CONSTRAINT battle_keywords_rules_uk_check
    CHECK (rules_uk IS NULL OR char_length(rules_uk) <= 300);

-- ── Etudes (challenges) and errands ─────────────────────────────────────────
ALTER TABLE battle_challenges
    ADD COLUMN IF NOT EXISTS title_uk TEXT NOT NULL DEFAULT '',
    ADD COLUMN IF NOT EXISTS note_uk  TEXT;
ALTER TABLE battle_challenges DROP CONSTRAINT IF EXISTS battle_challenges_title_uk_check;
ALTER TABLE battle_challenges ADD CONSTRAINT battle_challenges_title_uk_check
    CHECK (char_length(title_uk) <= 80);
ALTER TABLE battle_challenges DROP CONSTRAINT IF EXISTS battle_challenges_note_uk_check;
ALTER TABLE battle_challenges ADD CONSTRAINT battle_challenges_note_uk_check
    CHECK (note_uk IS NULL OR char_length(note_uk) <= 400);

ALTER TABLE battle_errands
    ADD COLUMN IF NOT EXISTS title_uk TEXT NOT NULL DEFAULT '',
    ADD COLUMN IF NOT EXISTS note_uk  TEXT;
ALTER TABLE battle_errands DROP CONSTRAINT IF EXISTS battle_errands_title_uk_check;
ALTER TABLE battle_errands ADD CONSTRAINT battle_errands_title_uk_check
    CHECK (char_length(title_uk) <= 120);
ALTER TABLE battle_errands DROP CONSTRAINT IF EXISTS battle_errands_note_uk_check;
ALTER TABLE battle_errands ADD CONSTRAINT battle_errands_note_uk_check
    CHECK (note_uk IS NULL OR char_length(note_uk) <= 400);

-- ── Studio: races brought by people ─────────────────────────────────────────
ALTER TABLE studio_races
    ADD COLUMN IF NOT EXISTS name_uk TEXT NOT NULL DEFAULT '',
    ADD COLUMN IF NOT EXISTS note_uk TEXT;
ALTER TABLE studio_races DROP CONSTRAINT IF EXISTS studio_races_name_uk_check;
ALTER TABLE studio_races ADD CONSTRAINT studio_races_name_uk_check
    CHECK (char_length(name_uk) <= 60);
ALTER TABLE studio_races DROP CONSTRAINT IF EXISTS studio_races_note_uk_check;
ALTER TABLE studio_races ADD CONSTRAINT studio_races_note_uk_check
    CHECK (note_uk IS NULL OR char_length(note_uk) <= 200);

-- ── The language a person worked or asked in: now one of three ──────────────
-- Letters and notes are written in this language, so a Ukrainian reader gets a
-- Ukrainian letter. The old checks only knew two.
ALTER TABLE tale_sequel_wishes DROP CONSTRAINT IF EXISTS tale_sequel_wishes_lang_check;
ALTER TABLE tale_sequel_wishes ADD CONSTRAINT tale_sequel_wishes_lang_check
    CHECK (lang IN ('en', 'ru', 'uk'));
ALTER TABLE tale_poll_votes DROP CONSTRAINT IF EXISTS tale_poll_votes_lang_check;
ALTER TABLE tale_poll_votes ADD CONSTRAINT tale_poll_votes_lang_check
    CHECK (lang IN ('en', 'ru', 'uk'));
ALTER TABLE tale_letters DROP CONSTRAINT IF EXISTS tale_letters_lang_check;
ALTER TABLE tale_letters ADD CONSTRAINT tale_letters_lang_check
    CHECK (lang IN ('en', 'ru', 'uk'));
ALTER TABLE tale_notes DROP CONSTRAINT IF EXISTS tale_notes_lang_check;
ALTER TABLE tale_notes ADD CONSTRAINT tale_notes_lang_check
    CHECK (lang IN ('en', 'ru', 'uk'));
ALTER TABLE tale_book_notes DROP CONSTRAINT IF EXISTS tale_book_notes_lang_check;
ALTER TABLE tale_book_notes ADD CONSTRAINT tale_book_notes_lang_check
    CHECK (lang IN ('en', 'ru', 'uk'));
ALTER TABLE telegram_subscribers DROP CONSTRAINT IF EXISTS telegram_subscribers_lang_check;
ALTER TABLE telegram_subscribers ADD CONSTRAINT telegram_subscribers_lang_check
    CHECK (lang IN ('en', 'ru', 'uk'));
ALTER TABLE studio_races DROP CONSTRAINT IF EXISTS studio_races_lang_check;
ALTER TABLE studio_races ADD CONSTRAINT studio_races_lang_check
    CHECK (lang IN ('en', 'ru', 'uk'));
ALTER TABLE studio_cards DROP CONSTRAINT IF EXISTS studio_cards_lang_check;
ALTER TABLE studio_cards ADD CONSTRAINT studio_cards_lang_check
    CHECK (lang IN ('en', 'ru', 'uk'));
ALTER TABLE studio_motions DROP CONSTRAINT IF EXISTS studio_motions_lang_check;
ALTER TABLE studio_motions ADD CONSTRAINT studio_motions_lang_check
    CHECK (lang IN ('en', 'ru', 'uk'));

-- ── Ukrainian for what the house itself wrote ──────────────────────────────
-- The errands that were seeded, and the siege gear, are written by the
-- project, not by the keeper, so they get their Ukrainian here. Only a row that
-- still carries the seeded Russian is touched, and only while its Ukrainian is
-- empty: an errand the keeper has rewritten, or already translated, is hers.
UPDATE battle_errands SET title_uk = 'Зайти в гру',
       note_uk = 'За те, що зазирнули сьогодні. Пропустіть день — нічого не втратите.'
 WHERE slug = 'came-by' AND title_uk = '' AND title_ru = 'Зайти в игру';
UPDATE battle_errands SET title_uk = 'Переглянути роботи',
       note_uk = 'Відкрийте п’ять робіт в архіві.'
 WHERE slug = 'look-around' AND title_uk = '' AND title_ru = 'Посмотреть работы';
UPDATE battle_errands SET title_uk = 'Відзначити роботу, що сподобалася',
       note_uk = 'Позначте сердечком роботу, яка сподобалася.'
 WHERE slug = 'closer-look' AND title_uk = '' AND title_ru = 'Отметить понравившуюся работу';
UPDATE battle_errands SET title_uk = 'Прочитати історію',
       note_uk = 'Дочитайте одну історію до кінця.'
 WHERE slug = 'read-a-tale' AND title_uk = '' AND title_ru = 'Прочитать историю';
UPDATE battle_errands SET title_uk = 'Залишити відгук про роботу',
       note_uk = 'Напишіть, що ви думаєте про роботу. Зарахується, коли автор його схвалить.'
 WHERE slug = 'leave-impression' AND title_uk = '' AND title_ru = 'Оставить отзыв о работе';
UPDATE battle_errands SET title_uk = 'Взяти першу карту',
       note_uk = 'Будь-яку карту з полиці.'
 WHERE slug = 'first-card' AND title_uk = '' AND title_ru = 'Взять первую карту';
UPDATE battle_errands SET title_uk = 'Зібрати колоду',
       note_uk = 'Шість карт: три на полі, три в руці.'
 WHERE slug = 'lay-the-table' AND title_uk = '' AND title_ru = 'Собрать колоду';
UPDATE battle_errands SET title_uk = 'Дограти бій до кінця',
       note_uk = 'Перемога чи поразка — важливо дограти до кінця.'
 WHERE slug = 'finish-a-study' AND title_uk = '' AND title_ru = 'Доиграть бой до конца';
UPDATE battle_errands SET title_uk = 'Підвищити рівень карти',
       note_uk = 'Рівень не впливає на бій — це позначка про те, що карта давно у вас.'
 WHERE slug = 'raise-a-card' AND title_uk = '' AND title_ru = 'Повысить уровень карты';
UPDATE battle_errands SET title_uk = 'Зібрати три карти',
       note_uk = 'Три власні, а не тимчасові.'
 WHERE slug = 'three-cards' AND title_uk = '' AND title_ru = 'Собрать три карты';
UPDATE battle_errands SET title_uk = 'Залишити відгук, який сподобається авторові',
       note_uk = 'Нагороду видає автор, якщо визнає за потрібне.'
 WHERE slug = 'an-impression' AND title_uk = '' AND title_ru = 'Оставить отзыв, который понравится автору';
UPDATE battle_errands SET title_uk = 'Прийти на показ',
       note_uk = 'Показ, що відбувся.'
 WHERE slug = 'a-showing' AND title_uk = '' AND title_ru = 'Прийти на показ';
UPDATE battle_errands SET title_uk = 'Замовити роботу',
       note_uk = 'Найрідкісніша нагорода і найбільша.'
 WHERE slug = 'a-commission' AND title_uk = '' AND title_ru = 'Заказать работу';

UPDATE battle_cards SET title_uk = 'Фортечний мур', type_uk = 'Споруда',
       effect_uk = 'Не ходить і не б’є. Тримає прохід, поки ціла.'
 WHERE slug = 'siege-wall' AND title_uk = '' AND title_ru = 'Крепостная стена';
UPDATE battle_cards SET title_uk = 'Сторожова вежа', type_uk = 'Споруда',
       effect_uk = 'Не ходить. Стріляє на три клітини, через мури.'
 WHERE slug = 'siege-tower' AND title_uk = '' AND title_ru = 'Сторожевая башня';
UPDATE battle_cards SET title_uk = 'Таран', type_uk = 'Облога',
       effect_uk = 'Б’є повз броню.'
 WHERE slug = 'siege-ram' AND title_uk = '' AND title_ru = 'Таран';
