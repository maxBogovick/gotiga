-- Аукцион: вещь уходит с молотка.
--
-- Два случая, и механизм у них ОДИН. Первый — человек сам решил отдать вещь
-- тому, кто больше даст. Второй, ради которого аукцион и заведён
-- (`STUDIO.md` §9): человек ушёл из дома, и всё ценное, что у него было,
-- уходит с молотка, а вырученное СГОРАЕТ. У такого лота продавца нет вовсе
-- (`seller_id IS NULL`) — потому что его и правда нет, и платить некому.
--
-- Ставка — НЕ ПЛАТЁЖ. С неё ничего не списывается: пока идут торги, деньги у
-- человека на руках, и он волен потратить их на другое. Поэтому платит
-- победитель В МОМЕНТ ЗАКРЫТИЯ, и если платить нечем — лот уходит следующему,
-- кто может. Списывать ставку сразу значило бы запирать пыль на неделю у всех,
-- кто просто участвовал.
CREATE TABLE IF NOT EXISTS auctions (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    kind        TEXT NOT NULL CHECK (kind IN ('license', 'copy')),
    subject_id  UUID NOT NULL,
    -- Пусто — вещь ушедшего: продавца нет, вырученное сгорает.
    seller_id   UUID REFERENCES users(id) ON DELETE SET NULL,
    -- Ушёл ли хозяин из дома. Отдельным словом, а не выводом из пустого
    -- продавца: продавец пустеет и когда учётную запись просто удалили, и знать
    -- об этом должен тот, кто решает, кому платить.
    estate      BOOLEAN NOT NULL DEFAULT FALSE,
    start_price INTEGER NOT NULL CHECK (start_price > 0),
    currency    TEXT NOT NULL CHECK (currency IN ('dust', 'feed')),
    ends_at     TIMESTAMPTZ NOT NULL,
    state       TEXT NOT NULL CHECK (state IN ('open', 'done', 'void')) DEFAULT 'open',
    winner_id   UUID REFERENCES users(id) ON DELETE SET NULL,
    final_price INTEGER,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    closed_at   TIMESTAMPTZ
);

CREATE INDEX IF NOT EXISTS auctions_open_idx ON auctions (ends_at) WHERE state = 'open';

-- Ставки. Хранятся ВСЕ, а не только высшая: если победитель не смог заплатить,
-- лот уходит следующему, кто может, — а «следующего» неоткуда взять, когда в
-- доме записана одна цифра.
CREATE TABLE IF NOT EXISTS auction_bids (
    id         UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    auction_id UUID NOT NULL REFERENCES auctions(id) ON DELETE CASCADE,
    bidder_id  UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    amount     INTEGER NOT NULL CHECK (amount > 0),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS auction_bids_idx ON auction_bids (auction_id, amount DESC, created_at);
