-- Страница работы на Etsy. Кнопка «Приобрести» ведёт сюда; пусто — в магазин.
ALTER TABLE figurines ADD COLUMN IF NOT EXISTS etsy_url TEXT;
