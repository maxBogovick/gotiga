-- Экземпляры и рынок — ПУСТЫМИ, заранее.
--
-- Ни одна строка этого файла сегодня не читается кодом. Он здесь потому, что
-- владение придётся переписывать: `battle_owned_cards (user_id, card_id)`
-- не может выразить ни «у меня два экземпляра», ни «вот этот самый, номер 7»,
-- а без этого правило «продал — у тебя больше нет» невыразимо вовсе.
--
-- Мигрировать пустую таблицу — час. Мигрировать живую, где у людей уже куплены
-- карты, — риск потерять чужие покупки. Тот же довод, по которому в этом доме
-- заведён заранее `level_price_dust`, и по которому `battle_wallet_entries`
-- стояла пустой до первой покупки.

-- Лицензия: право носить рамку. Продаётся ИМЕННО ОНА, а не рамка: продать
-- «рамку» значило бы раздеть в момент сделки все карты автора, её носившие.
CREATE TABLE IF NOT EXISTS frame_licenses (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    frame_id    UUID NOT NULL REFERENCES studio_frames(id) ON DELETE CASCADE,
    owner_id    UUID REFERENCES users(id) ON DELETE SET NULL,
    -- Номер из тиража: «№7 из 200». То, чего у другого нет.
    serial      INTEGER NOT NULL,
    origin      TEXT NOT NULL CHECK (origin IN ('author', 'bought', 'traded', 'prize', 'gift')),
    -- Выставлена на продажу — заперта: ею нельзя играть и нельзя продать
    -- второму. Эскроу без слова «эскроу».
    locked_by   UUID,
    acquired_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT frame_licenses_serial UNIQUE (frame_id, serial)
);

CREATE INDEX IF NOT EXISTS frame_licenses_owner_idx ON frame_licenses (owner_id);

-- Экземпляр карты: это и есть «вот эта самая карта». Продажа — смена
-- владельца в одной строке; ничего не копируется, ничего не исчезает.
CREATE TABLE IF NOT EXISTS card_copies (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    card_id     UUID NOT NULL REFERENCES battle_cards(id) ON DELETE CASCADE,
    owner_id    UUID REFERENCES users(id) ON DELETE SET NULL,
    serial      INTEGER,
    -- Уровень принадлежит ЭТОМУ экземпляру — так и было задумано с самого
    -- начала. Прокачанная карта объективно дороже непрокачанной, и рынку есть
    -- чем торговать, кроме редкости.
    level       SMALLINT NOT NULL CHECK (level BETWEEN 1 AND 5) DEFAULT 1,
    origin      TEXT NOT NULL CHECK (origin IN ('bought', 'traded', 'prize', 'gift')),
    locked_by   UUID,
    acquired_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    seen_at     TIMESTAMPTZ
);

CREATE INDEX IF NOT EXISTS card_copies_owner_idx ON card_copies (owner_id);
CREATE UNIQUE INDEX IF NOT EXISTS card_copies_serial_idx
    ON card_copies (card_id, serial) WHERE serial IS NOT NULL;

-- Объявление. Цена в одной из двух валют дома; комиссия дома сгорает — это
-- единственный сток валюты, без него за месяц цены перестанут что-то значить.
CREATE TABLE IF NOT EXISTS market_listings (
    id         UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    kind       TEXT NOT NULL CHECK (kind IN ('license', 'copy')),
    subject_id UUID NOT NULL,
    seller_id  UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    price      INTEGER NOT NULL CHECK (price > 0),
    currency   TEXT NOT NULL CHECK (currency IN ('dust', 'feed')),
    state      TEXT NOT NULL CHECK (state IN ('open', 'sold', 'withdrawn')) DEFAULT 'open',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    closed_at  TIMESTAMPTZ
);

CREATE INDEX IF NOT EXISTS market_listings_open_idx ON market_listings (created_at) WHERE state = 'open';

-- Тираж карты. NULL — печатается сколько угодно (лавка дома); число —
-- ограниченный тираж; 1 — уникальная, приз победителю сезона.
--
-- Без ограниченного тиража рынка не существует: то, что каждый может взять в
-- лавке, никто не купит у человека.
ALTER TABLE battle_cards ADD COLUMN IF NOT EXISTS edition_size INTEGER
    CHECK (edition_size IS NULL OR edition_size > 0);
ALTER TABLE battle_cards ADD COLUMN IF NOT EXISTS minted INTEGER NOT NULL DEFAULT 0;

-- Тираж лицензий утверждённой рамы и коридор её цены — назначает хозяин при
-- утверждении.
ALTER TABLE studio_frames ADD COLUMN IF NOT EXISTS edition_size INTEGER
    CHECK (edition_size IS NULL OR edition_size > 0);
ALTER TABLE studio_frames ADD COLUMN IF NOT EXISTS approved_at TIMESTAMPTZ;
