-- The cancel token generator now produces 19-char tokens (XXXX-XXXX-XXXX-XXXX),
-- but figurine_bookings.cancel_token was originally VARCHAR(9), causing
-- "value too long for type character varying(9)" (SQLSTATE 22001) on booking.
-- Widen it to TEXT to match orders.cancel_token and figurine_waitlist.cancel_token.
ALTER TABLE figurine_bookings ALTER COLUMN cancel_token TYPE TEXT;
