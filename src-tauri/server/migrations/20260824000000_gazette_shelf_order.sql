-- Порядок на полке небылиц. Полка расставляется руками, поэтому вес листа
-- на ней не выводится из даты: NULL значит «ещё не расставлено» — такой лист
-- падает в конец и сортируется по времени, как обычный листок вестника.
ALTER TABLE gazette_leaves ADD COLUMN IF NOT EXISTS shelf_order INTEGER;

CREATE INDEX IF NOT EXISTS gazette_leaves_shelf_idx
    ON gazette_leaves (shelf_order NULLS LAST,
                       COALESCE(published_at, scheduled_at, created_at) DESC)
    WHERE kind = 'tale' AND status IN ('published', 'scheduled');
