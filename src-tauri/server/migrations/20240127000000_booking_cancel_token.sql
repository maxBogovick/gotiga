-- Add cancel_token to figurine_bookings for user-side self-cancellation
ALTER TABLE figurine_bookings ADD COLUMN IF NOT EXISTS cancel_token VARCHAR(9);

-- Backfill existing rows
UPDATE figurine_bookings
SET cancel_token = UPPER(SUBSTRING(REPLACE(gen_random_uuid()::text, '-', ''), 1, 4))
                || '-'
                || UPPER(SUBSTRING(REPLACE(gen_random_uuid()::text, '-', ''), 5, 4))
WHERE cancel_token IS NULL;

ALTER TABLE figurine_bookings ALTER COLUMN cancel_token SET NOT NULL;
CREATE UNIQUE INDEX IF NOT EXISTS idx_booking_cancel_token ON figurine_bookings(cancel_token);
