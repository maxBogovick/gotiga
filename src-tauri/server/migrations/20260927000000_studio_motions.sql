-- Свои движения: человек приносит ТАКТ, дом ставит его в свод.
--
-- Движение — сочетание готовых жестов, а не новый жест: список тел закрыт по
-- той же причине, по которой закрыт список глаголов способностей — движок не
-- умеет ничего сверх них, и «новое движение» это новая комбинация. Поэтому
-- принесённое человеком безопасно ровно так же, как принесённая им способность.
--
-- Тело — ОДИН столбец JSON, буква в букву `Motion` из свода дома: тот же разбор,
-- те же зажимы (`normalize_motion`), тот же отрисовщик. Вторая схема движения
-- разошлась бы с первой на первом же новом поле жеста.
--
-- Слова состояния — те же четыре, что у карт и родов.
CREATE TABLE IF NOT EXISTS studio_motions (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    owner_id    UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    -- Имя движения внутри тела (`body->>'id'`): на него показывают карта, раса
    -- и порядок в ящике, и второго места для него быть не должно.
    body        JSONB NOT NULL,
    status      TEXT NOT NULL CHECK (status IN ('draft', 'shown', 'withdrawn', 'taken'))
                DEFAULT 'draft',
    keeper_word TEXT,
    approved_at TIMESTAMPTZ,
    -- Каким именем движение встало в свод дома. Не ссылка: свод — это строка
    -- настроек, а не таблица, и внешнему ключу здесь не за что держаться.
    house_id    TEXT,
    lang        TEXT NOT NULL DEFAULT 'ru' CHECK (lang IN ('ru', 'en')),
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    -- Забор от мегабайтного тела: движение с полосой кадров — это адреса
    -- картинок и числа, единицы килобайт.
    CONSTRAINT studio_motions_body_size CHECK (octet_length(body::text) <= 40000)
);

CREATE INDEX IF NOT EXISTS studio_motions_owner_idx ON studio_motions (owner_id, updated_at DESC);
CREATE INDEX IF NOT EXISTS studio_motions_queue_idx ON studio_motions (updated_at) WHERE status = 'shown';
-- Одно имя движения на человека: стол правит ящик целиком и сохраняет его
-- разом, и два ряда с одним именем — это движение, которое он однажды потеряет.
CREATE UNIQUE INDEX IF NOT EXISTS studio_motions_name_idx
    ON studio_motions (owner_id, (body->>'id'));
