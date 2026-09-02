-- Поручения.
--
-- Поручение не создаёт дело — оно НАЗЫВАЕТ ЗАРАНЕЕ то, что дом и так считал
-- молча (`TASKS-BATTLE-ERRANDS.md` §0). Пыль за прочитанную небылицу капала и
-- вчера; человек просто узнавал об этом задним числом, из строки «начислено с
-- прошлого раза».
--
-- Платит эта таблица из той же дописываемой книги (`battle_wallet_entries`),
-- тем же способом, что этюд и внимание: строка с `idem_key`. Отсюда даром
-- достаётся «ровно однажды» для ЛЮБОГО периода — разовое, ежедневное и
-- недельное отличаются только тем, как из slug'а и времени собирается ключ.
-- Ни таблицы выданных наград, ни поля «когда в последний раз», ни ночной
-- задачи не нужно.
CREATE TABLE IF NOT EXISTS battle_errands (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    -- Из него собирается ключ книги, и потому он НЕИЗМЕНЕН после первой
    -- выплаты: переименование slug'а — это новое поручение, за которое
    -- заплатят второй раз. Стол хранителя запирает поле.
    slug        TEXT NOT NULL,
    title_en    TEXT NOT NULL CHECK (char_length(title_en) BETWEEN 1 AND 120),
    title_ru    TEXT NOT NULL CHECK (char_length(title_ru) BETWEEN 1 AND 120),
    note_en     TEXT CHECK (note_en IS NULL OR char_length(note_en) <= 400),
    note_ru     TEXT CHECK (note_ru IS NULL OR char_length(note_ru) <= 400),
    -- Слово из словаря условий (`BATTLE-ERRANDS.md` §3). CHECK'а здесь нет
    -- намеренно: словарь живёт в коде, где у каждого условия есть чистая
    -- функция, и миграция, повторяющая его вторым списком, разошлась бы с ним
    -- на первом же добавлении. Неизвестное слово ничего не платит.
    rule        TEXT NOT NULL,
    -- «Сколько». Единица — это условие «да/нет».
    threshold   INTEGER NOT NULL DEFAULT 1 CHECK (threshold > 0),
    currency    TEXT NOT NULL CHECK (currency IN ('dust', 'feed')),
    amount      INTEGER NOT NULL CHECK (amount > 0),
    -- Умолчание `once` — не вкус, а ограничение: повторяющееся поручение даёт
    -- повод прийти РАДИ пыли, и его надо выбирать руками.
    period      TEXT NOT NULL DEFAULT 'once'
                    CHECK (period IN ('once', 'daily', 'weekly', 'window')),
    starts_at   TIMESTAMPTZ,
    ends_at     TIMESTAMPTZ,
    status      TEXT NOT NULL DEFAULT 'draft'
                    CHECK (status IN ('draft', 'published')),
    sort_order  INTEGER,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE UNIQUE INDEX IF NOT EXISTS battle_errands_slug_idx ON battle_errands (slug);
CREATE INDEX IF NOT EXISTS battle_errands_shelf_idx
    ON battle_errands (sort_order NULLS LAST, created_at)
    WHERE status = 'published';

-- Тропа: первый час в комнате.
--
-- Данными, а не кодом: хранителю их править, а не программисту. Числа выбраны
-- против измеренных стен полки (`TASKS-BATTLE-ERRANDS.md` §4) — вся тропа даёт
-- 90 пыли, и с даром в 10 это две карты за первый вечер (12 и 40) и сдача в
-- сторону третьей (75). Дорогая карта с тропы не берётся: на неё нужно время в
-- доме, и это ровно та кривая, ради которой всё считалось.
--
-- «Оглядеться» — ПЯТЬ работ, а не двадцать: двадцать — это уже добыча, и
-- правило комнаты нарушено.
INSERT INTO battle_errands
    (slug, title_en, title_ru, note_en, note_ru, rule, threshold, currency, amount, status, sort_order)
VALUES
    ('look-around', 'Look around', 'Оглядеться',
     'Open five works in the archive.', 'Откройте пять работ в архиве.',
     'works_seen', 5, 'dust', 5, 'published', 10),
    ('closer-look', 'Look closer', 'Приглядеться',
     'Leave a heart on a work you liked.', 'Поставьте сердечко работе, которая понравилась.',
     'works_liked', 1, 'dust', 5, 'published', 20),
    ('read-a-tale', 'Read a tall tale', 'Прочесть небылицу',
     'Read one tale to the last line.', 'Дочитайте одну небылицу до последней строки.',
     'tales_read', 1, 'dust', 5, 'published', 30),
    ('leave-impression', 'Leave an impression', 'Оставить впечатление',
     'Write what you thought of a work. Counts once the author has read it.',
     'Напишите, что вы думаете о работе. Засчитается, когда автор её прочтёт.',
     'comments_left', 1, 'dust', 10, 'published', 40),
    ('first-card', 'Take your first card', 'Взять первую карту',
     'Any card from the shelf.', 'Любую карту с полки.',
     'cards_owned', 1, 'dust', 10, 'published', 50),
    ('lay-the-table', 'Lay out your deck', 'Разложить стол',
     'Six places: three on the field, three in hand.', 'Шесть мест: три на поле, три в руке.',
     'deck_laid', 1, 'dust', 10, 'published', 60),
    ('finish-a-study', 'Play a study to the end', 'Довести этюд до конца',
     'Win or lose — what counts is reaching the end.', 'Победа или нет — важно дойти до конца.',
     'matches_finished', 1, 'dust', 15, 'published', 70),
    ('raise-a-card', 'Raise a card a step', 'Поднять карту на ступень',
     'A level is a mark of time, not of strength.', 'Уровень — отметка о времени, а не о силе.',
     'card_level', 2, 'dust', 10, 'published', 80),
    ('three-cards', 'Gather three cards', 'Собрать три карты',
     'Three of your own, not lent ones.', 'Три своих, а не временных.',
     'cards_owned', 3, 'dust', 20, 'published', 90)
ON CONFLICT (slug) DO NOTHING;
