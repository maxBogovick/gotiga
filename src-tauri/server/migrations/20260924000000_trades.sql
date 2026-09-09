-- Мена: вещь за вещь, без денег.
--
-- Мена предлагается ПРОТИВ ОБЪЯВЛЕНИЯ, а не «кому угодно за что угодно», и это
-- не упрощение, а единственный честный вход: чтобы предложить человеку мену,
-- надо сперва увидеть, что у него есть, — а места, где видно чужое собрание, в
-- доме нет и заводить его незачем. Прилавок — это и есть «вот моя вещь, она
-- отдаётся»; мена говорит «отдам за своё, а не за пыль».
--
-- Долю дома мена не платит: пыли она не создаёт, и брать её не с чего.
CREATE TABLE IF NOT EXISTS trades (
    id         UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    listing_id UUID NOT NULL REFERENCES market_listings(id) ON DELETE CASCADE,
    -- Кто предлагает. Кому — говорит объявление: его продавец.
    from_id    UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    state      TEXT NOT NULL CHECK (state IN ('offered', 'taken', 'refused', 'gone'))
               DEFAULT 'offered',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    closed_at  TIMESTAMPTZ
);

CREATE INDEX IF NOT EXISTS trades_listing_idx ON trades (listing_id) WHERE state = 'offered';
CREATE INDEX IF NOT EXISTS trades_from_idx ON trades (from_id, created_at DESC);

-- Что предлагают. Список, а не одна вещь: мена «моя пара за твою одну» —
-- обычное дело, и запретить её значило бы заставить менять по одной, где
-- половина мены уже подарок.
CREATE TABLE IF NOT EXISTS trade_items (
    trade_id   UUID NOT NULL REFERENCES trades(id) ON DELETE CASCADE,
    kind       TEXT NOT NULL CHECK (kind IN ('license', 'copy')),
    subject_id UUID NOT NULL,
    PRIMARY KEY (trade_id, kind, subject_id)
);

-- Мена закрывает объявление СВОИМ словом: «продано» про вещь, за которую не
-- дали ни пылинки, — это число, которое врёт молча, а по журналу лавки потом
-- считают обороты.
ALTER TABLE market_listings DROP CONSTRAINT IF EXISTS market_listings_state_check;
ALTER TABLE market_listings ADD CONSTRAINT market_listings_state_check
    CHECK (state IN ('open', 'sold', 'withdrawn', 'traded'));
