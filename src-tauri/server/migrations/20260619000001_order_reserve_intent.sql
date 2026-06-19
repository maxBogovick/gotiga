ALTER TYPE order_mode ADD VALUE IF NOT EXISTS 'reserve';

DO $$
BEGIN
    CREATE TYPE reserve_status AS ENUM (
        'requested',
        'reviewing',
        'terms_sent',
        'confirmed',
        'declined',
        'expired'
    );
EXCEPTION
    WHEN duplicate_object THEN NULL;
END $$;

ALTER TABLE orders
    ADD COLUMN IF NOT EXISTS reserve_status reserve_status,
    ADD COLUMN IF NOT EXISTS reserve_expires_at DATE,
    ADD COLUMN IF NOT EXISTS admin_terms_note TEXT,
    ADD COLUMN IF NOT EXISTS invoice_note TEXT;

CREATE INDEX IF NOT EXISTS orders_mode_idx ON orders (mode);
CREATE INDEX IF NOT EXISTS orders_reserve_status_idx ON orders (reserve_status);
