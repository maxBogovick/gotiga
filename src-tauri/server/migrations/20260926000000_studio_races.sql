-- Свои роды: человек приносит РОД, дом ставит его в словарь.
--
-- Род — не карта и не рама: это словарная строка, которую носят чужие карты.
-- Отсюда и единственное отличие от карт: у рода нет содержимого, которое можно
-- взвесить, — есть имя, слово о нём и значок. Решает хозяин глазами.
--
-- Слова состояния — ТЕ ЖЕ ЧЕТЫРЕ, что у карт (`STUDIO-CARDS-REVIEW.md`): три
-- про автора и `taken` про дом. Третий набор слов для третьей вещи был бы
-- третьим заходом на грабли, которые дом уже прошёл дважды.
CREATE TABLE IF NOT EXISTS studio_races (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    owner_id    UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    name_en     TEXT NOT NULL CHECK (char_length(name_en) BETWEEN 1 AND 60),
    name_ru     TEXT NOT NULL CHECK (char_length(name_ru) BETWEEN 1 AND 60),
    note_en     TEXT CHECK (note_en IS NULL OR char_length(note_en) <= 200),
    note_ru     TEXT CHECK (note_ru IS NULL OR char_length(note_ru) <= 200),
    -- Значок — из своего ящика: второго склада для этого заводить незачем.
    icon_url    TEXT,
    status      TEXT NOT NULL CHECK (status IN ('draft', 'shown', 'withdrawn', 'taken'))
                DEFAULT 'draft',
    keeper_word TEXT,
    approved_at TIMESTAMPTZ,
    -- Что из работы вышло: строка словаря дома. `ON DELETE SET NULL` — род
    -- убрали из словаря, память о принесённом осталась.
    race_id     UUID REFERENCES battle_races(id) ON DELETE SET NULL,
    lang        TEXT NOT NULL DEFAULT 'ru' CHECK (lang IN ('ru', 'en')),
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS studio_races_owner_idx ON studio_races (owner_id, updated_at DESC);
CREATE INDEX IF NOT EXISTS studio_races_queue_idx ON studio_races (updated_at) WHERE status = 'shown';

-- Кто придумал род. Как и у карты, печатается тихо и НЕ на самой карте: на
-- листе взятия, там же, где имя придумавшего карту.
ALTER TABLE battle_races ADD COLUMN IF NOT EXISTS credit_name TEXT;
