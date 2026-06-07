ALTER TABLE figurine_bookings ADD COLUMN IF NOT EXISTS requester_phone TEXT;
ALTER TABLE orders ADD COLUMN IF NOT EXISTS requester_phone TEXT;
