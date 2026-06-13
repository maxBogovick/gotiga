-- "Notify me" subscriptions get an unguessable token so anonymous visitors
-- get a receipt and can stop notifications. Only notify-mode orders carry one;
-- request/question orders leave it NULL.
ALTER TABLE orders
    ADD COLUMN IF NOT EXISTS cancel_token TEXT;

CREATE UNIQUE INDEX IF NOT EXISTS idx_orders_cancel_token
    ON orders(cancel_token) WHERE cancel_token IS NOT NULL;
