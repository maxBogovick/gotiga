-- Свои карты: человек приносит СОДЕРЖИМОЕ, дом даёт оформление.
--
-- Почему не столбец `author_id` в `battle_cards` (`BATTLE-PRESS.md` §2, решение
-- живо): `battle_cards` читают полка, покупка, владение, колода, испытания,
-- движок и весы, и строка непроверенной чужой работы отделена там от «её можно
-- купить» одним условием в одном запросе. Условия забывают. Отдельная таблица
-- не забывается никогда. Вдобавок у работы нет половины полей карты — слуга,
-- цены, `figurine_id`, — и ради строк, которые ещё не карты, пришлось бы снять
-- заборы со всей таблицы.
--
-- Тело — ОДИН столбец JSON, буква в букву `SaveBattleCardRequest`: тот самый
-- запрос, который стол хозяина уже посылает. Тридцать столбцов были бы второй
-- схемой карты, и каждая миграция схемы требовала бы близнеца; забытый близнец
-- — это поле, которое человек назначил, а приём молча потерял. Даром достаются
-- весы, нормализация и отказ: они уже принимают ровно этот запрос.
--
-- Слова состояния — ТЕ ЖЕ ТРИ, что у рам, и это не единообразие ради красоты:
-- `STUDIO-REVIEW.md` показал, чем кончаются пять слов, в которых перепутаны три
-- разных вопроса. У карт вопросы те же, и второй набор слов был бы вторым
-- заходом на те же грабли.
CREATE TABLE IF NOT EXISTS studio_cards (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    owner_id    UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    -- Имени отдельным столбцом НЕТ намеренно: имя берётся `body->>'titleRu'`.
    -- Два места, где написано одно имя, однажды разойдутся.
    body        JSONB NOT NULL,
    status      TEXT NOT NULL CHECK (status IN ('draft', 'shown', 'withdrawn'))
                DEFAULT 'draft',
    -- Решение хозяина — отметками, а не словами состояния: состояние про то,
    -- что может сделать АВТОР.
    admitted_at TIMESTAMPTZ,
    admit_word  TEXT,
    approved_at TIMESTAMPTZ,
    -- Что из работы вышло. `ON DELETE SET NULL`: карту сняли с полки — память о
    -- принесённом осталась.
    card_id     UUID REFERENCES battle_cards(id) ON DELETE SET NULL,
    -- На каком языке человек работал: записки ему пишутся на нём, а не на том,
    -- который открыт у хозяина.
    lang        TEXT NOT NULL DEFAULT 'ru' CHECK (lang IN ('ru', 'en')),
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    -- Забор от мегабайтного тела: карта со всеми чертами и способностями
    -- укладывается в единицы килобайт.
    CONSTRAINT studio_cards_body_size CHECK (octet_length(body::text) <= 20000)
);

CREATE INDEX IF NOT EXISTS studio_cards_owner_idx
    ON studio_cards (owner_id, updated_at DESC);
-- Очередь допуска: отдано и хозяин ещё не смотрел. Самое старое сверху.
CREATE INDEX IF NOT EXISTS studio_cards_admission_idx
    ON studio_cards (updated_at) WHERE status = 'shown' AND admitted_at IS NULL;

-- Автограф. Печатается строкой на листе взятия, а не на самой карте: на карте
-- нет свободного места, а лицо карты — опись, в которую не втискивают
-- шестнадцатую строку ради служебного факта. У карт дома — NULL.
ALTER TABLE battle_cards ADD COLUMN IF NOT EXISTS credit_name TEXT;
