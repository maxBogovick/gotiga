-- Опыт «со сценой / без» (src/lib/sceneAb.ts): какой вариант страницы видел
-- читатель. Колонка своя, а не `internal_source` и не `utm_*`: те говорят,
-- откуда человек пришёл (`?src=telegram` у канала — самое ценное, что в них
-- лежит), и вариант, записанный поверх, стёр бы это.
--
-- NULL — страница без опыта, то есть почти все события. Вариант пишется как
-- `<опыт>:<плечо>` (`scene:on`, `scene:off`); ярлык от рук (`?scene=` в
-- адресе, для просмотра) не пишется вовсе, чтобы не мешать счёт.
ALTER TABLE figurine_analytics_events
    ADD COLUMN IF NOT EXISTS variant TEXT;

CREATE INDEX IF NOT EXISTS idx_figurine_analytics_events_variant
    ON figurine_analytics_events (event_date, variant)
    WHERE variant IS NOT NULL;
