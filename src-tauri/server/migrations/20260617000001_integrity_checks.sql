-- DB-level integrity guards that were previously only enforced (if at all) in
-- the application layer. NOT VALID skips checking pre-existing rows, so the
-- migration is safe on populated databases while still enforcing new writes.

-- Date ranges must be well-formed.
ALTER TABLE figurine_bookings
    ADD CONSTRAINT chk_booking_dates CHECK (starts_at <= ends_at) NOT VALID;
ALTER TABLE figurine_showings
    ADD CONSTRAINT chk_showing_dates CHECK (starts_at <= ends_at) NOT VALID;

-- Bound free-text commission fields so a client can't store unbounded blobs.
ALTER TABLE commissions
    ADD CONSTRAINT chk_commission_description_len CHECK (char_length(description) <= 5000) NOT VALID;
ALTER TABLE commissions
    ADD CONSTRAINT chk_commission_title_len CHECK (char_length(title) <= 200) NOT VALID;
