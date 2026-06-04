CREATE TYPE order_status AS ENUM ('new', 'seen', 'replied');
CREATE TYPE order_mode AS ENUM ('request', 'question', 'notify');

CREATE TABLE orders (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    figurine_id TEXT NOT NULL,
    figurine_name TEXT NOT NULL,
    requester_name TEXT NOT NULL DEFAULT '',
    requester_email TEXT NOT NULL,
    message     TEXT,
    mode        order_mode NOT NULL DEFAULT 'request',
    status      order_status NOT NULL DEFAULT 'new',
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX orders_status_idx ON orders (status);
CREATE INDEX orders_created_at_idx ON orders (created_at DESC);
