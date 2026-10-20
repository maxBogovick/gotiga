-- Канал объявляет и игру: новую карту на полке и новый этюд.
--
-- Прежние две строки рода (`work`, `leaf`) остаются как были. Карты и этюды,
-- уже стоящие на сайте к моменту выкладки, сервер отметит пропущенными сам при
-- первом проходе (ключ `telegram_channel_since_play`): иначе канал начал бы
-- игру со всей полки разом.
ALTER TABLE telegram_channel_posts DROP CONSTRAINT IF EXISTS telegram_channel_posts_kind_check;
ALTER TABLE telegram_channel_posts
    ADD CONSTRAINT telegram_channel_posts_kind_check
    CHECK (kind IN ('work', 'leaf', 'card', 'etude'));
