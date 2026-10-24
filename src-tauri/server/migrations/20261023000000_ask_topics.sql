-- Темы и страницы у записей каталога «Спросить сайт».
--
-- Когда вопросов много, список не может быть одной лентой: пустое поле
-- показывает вопросы про страницу, на которой человек стоит, а остальные лежат
-- по темам. Тема — одна из закрытого списка (названия на шести языках лежат в
-- словаре страницы, а не в базе: подпись темы не правят вместе с ответом),
-- страницы — где запись показывается первой.
--
-- Новая колонка с названием `topic` есть у каждой записи; запись без темы
-- сервер при сохранении относит к «site» (`ask::normalize`), чтобы вопрос не
-- пропал из списка.

ALTER TABLE ask_answers
    ADD COLUMN IF NOT EXISTS topic TEXT NOT NULL DEFAULT 'site'
        CHECK (topic IN ('works', 'order', 'rights', 'site', 'stories', 'play', 'studio')),
    ADD COLUMN IF NOT EXISTS pages JSONB NOT NULL DEFAULT '[]'::jsonb
        CHECK (jsonb_typeof(pages) = 'array');

UPDATE ask_answers SET topic = 'works'
 WHERE key IN ('materials', 'size', 'unique', 'care', 'author', 'data_count', 'data_new');
UPDATE ask_answers SET topic = 'order'
 WHERE key IN ('how_order', 'price', 'shipping', 'payment', 'commission', 'returns',
               'order_status', 'lead_time', 'discounts', 'reserve', 'gift', 'contact');
UPDATE ask_answers SET topic = 'stories'
 WHERE key IN ('tale_what', 'gazette_what', 'data_read');
UPDATE ask_answers SET topic = 'play'
 WHERE key IN ('play_rules', 'play_first', 'play_races', 'play_win', 'play_cards_get');
UPDATE ask_answers SET topic = 'site'
 WHERE key IN ('is_shop', 'privacy', 'account', 'forgot_signs', 'bot_nature', 'unsubscribe', 'data_start');

UPDATE ask_answers SET pages = '["work"]'::jsonb
 WHERE key IN ('how_order', 'price', 'reserve', 'size', 'materials', 'unique', 'shipping');
UPDATE ask_answers SET pages = '["tale"]'::jsonb WHERE key IN ('tale_what', 'data_read');
UPDATE ask_answers SET pages = '["play"]'::jsonb
 WHERE key IN ('play_rules', 'play_first', 'play_win', 'play_races');
UPDATE ask_answers SET pages = '["gazette"]'::jsonb WHERE key IN ('gazette_what', 'unsubscribe');
