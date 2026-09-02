-- Дела: поручения, которые дом НЕ платит сам.
--
-- Корм не оседает (`TASKS-BATTLES.md` §0.1). Его даёт хранитель руками — за
-- поступок в настоящем мире: состоявшийся показ, впечатление, заказ. Дом умеет
-- померить и показ, и заказ, и соблазн заплатить за них автоматически велик —
-- но тогда вторая монета перестаёт отличаться от первой, а весь смысл двух
-- монет в том, что одна за время, а другая за поступок.
--
-- Поэтому дело — это поручение, НАЗВАННОЕ ЗАРАНЕЕ и не выплачиваемое машиной.
-- Гость видит, что корм вообще бывает и за что; кнопки «получить» у него нет и
-- быть не может.
ALTER TABLE battle_errands
    ADD COLUMN IF NOT EXISTS by_hand BOOLEAN NOT NULL DEFAULT FALSE;

INSERT INTO battle_errands
    (slug, title_en, title_ru, note_en, note_ru, rule, threshold, currency, amount,
     period, status, by_hand, sort_order)
VALUES
    ('an-impression', 'Leave an impression the author keeps', 'Впечатление, которое автор оставит',
     'The author decides. Written by hand, given by hand.',
     'Решает автор. Написано рукой — и дано рукой.',
     'comments_left', 1, 'feed', 1, 'once', 'published', TRUE, 200),
    ('a-showing', 'Come to a showing', 'Прийти на показ',
     'A showing that actually happened.', 'Показ, который состоялся.',
     'bookings_done', 1, 'feed', 2, 'once', 'published', TRUE, 210),
    ('a-commission', 'Commission a work', 'Заказать работу',
     'The rarest of the three, and the largest.', 'Самое редкое из трёх — и самое крупное.',
     'orders_made', 1, 'feed', 5, 'once', 'published', TRUE, 220)
ON CONFLICT (slug) DO NOTHING;
