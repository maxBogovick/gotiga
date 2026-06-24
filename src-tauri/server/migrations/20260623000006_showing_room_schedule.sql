-- Day-of-week and calendar-date scheduling for a showing room (Task B).
--   open_days_mask : bitmask of allowed weekdays (bit0=Mon … bit6=Sun). NULL → every day.
--   open_month_day : "MM-DD" — opens every year on that date (recurring ritual). NULL → unused.
--   open_date_from/open_date_until : "YYYY-MM-DD" inclusive — a one-off date range. NULL → unused.
-- Annual (open_month_day) and the one-off range are mutually exclusive in the UI.
-- All NULL keeps a room a plain daily window, so existing rooms are unaffected.
ALTER TABLE showing_rooms ADD COLUMN IF NOT EXISTS open_days_mask INTEGER;
ALTER TABLE showing_rooms ADD COLUMN IF NOT EXISTS open_month_day TEXT;
ALTER TABLE showing_rooms ADD COLUMN IF NOT EXISTS open_date_from TEXT;
ALTER TABLE showing_rooms ADD COLUMN IF NOT EXISTS open_date_until TEXT;
