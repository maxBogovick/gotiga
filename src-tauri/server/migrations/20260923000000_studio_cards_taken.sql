-- Разбор `STUDIO-CARDS-REVIEW.md`: у взятого дома должно быть СВОЁ слово.
--
-- Замок утверждения носил чужое: работу переводили в `withdrawn`, а всё, что
-- дом разрешает снятому, он разрешал и взятому — править, показывать снова и
-- вечно висеть в очереди хозяина, откуда её нельзя ни утвердить, ни вернуть.
-- Ровно та же поломка, что была у рам (`STUDIO-REVIEW.md`), и по той же
-- причине: служебное состояние назвали существующим словом.
--
-- `taken` терминально: не правится, не показывается, не выбрасывается.
ALTER TABLE studio_cards DROP CONSTRAINT IF EXISTS studio_cards_status_check;

-- Уже утверждённые носят `withdrawn` — их и переводим: `approved_at` говорит,
-- что произошло, а состояние молчало.
UPDATE studio_cards SET status = 'taken' WHERE approved_at IS NOT NULL;

ALTER TABLE studio_cards ADD CONSTRAINT studio_cards_status_check
    CHECK (status IN ('draft', 'shown', 'withdrawn', 'taken'));

-- Допуск у карт снимается. Он не значил ничего, кроме ящика на столе хозяина:
-- допущенной карты не видит никто, в сезон она не идёт, а утвердить можно было
-- и мимо допуска — порядок держался кнопкой на странице. Решения у хозяина
-- два, и оба с последствиями: взять или вернуть со словом.
ALTER TABLE studio_cards DROP COLUMN IF EXISTS admitted_at;

-- Одно слово хозяина вместо поля, которое значило то допуск, то отказ.
ALTER TABLE studio_cards RENAME COLUMN admit_word TO keeper_word;

DROP INDEX IF EXISTS studio_cards_admission_idx;
-- Очередь: отдано и ещё не взято. Самое старое сверху.
CREATE INDEX IF NOT EXISTS studio_cards_queue_idx
    ON studio_cards (updated_at) WHERE status = 'shown';
