-- The admin form's "Series" field (editorial grouping — also the archive page's
-- filter axis) was never actually wired to a column: it round-tripped through
-- FigurineListItemDto/AdminFigurineAnalyticsListItem but every write silently
-- dropped it and every read hardcoded `series: None`. This gives it a real home.
ALTER TABLE figurines ADD COLUMN IF NOT EXISTS series TEXT;
